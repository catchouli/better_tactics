import { Chess } from '../deps/chess.js';
import { Chessground } from '../deps/chessground.min.js';

export class PuzzleBoard {
    constructor(container, config)
    {
        this._container = container;
        this._config = {};

        this._board = Chessground(this._container);

        this._premove = null;
        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;

        this._failed = false;

        this._moves = [];

        this._board_states = [this.initial_board_state()];
        this._seek_position = 0;

        this._player_color = 'w';
        this._computer_color = 'b';

        this.configure(Object.assign(this.default_config(), config ? config : {}));
    }

    initial_board_state() {
        return {
            fen: this._config.fen ? this._config.fen
                : 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',
            side_to_move: this.get_color_to_move(this._config.fen),
            highlight_move: null,
        };
    }

    push_board_state(last_move, side_to_move) {
        this.truncate_board_states();
        this._board_states.push({
            fen: this._game.fen(),
            highlight_move: last_move,
            side_to_move: side_to_move,
        });
        this._seek_position = this._board_states.length - 1;
    }

    // Truncate board states after the current one.
    truncate_board_states() {
        if (this._board_states.length > this._seek_position + 1) {
            this._board_states.length = this._seek_position + 1;
            this._seek_position = this.seek_max();

            if (this._config.on_seek) {
                this._config.on_seek();
            }
        }
    }

    get_color_to_move(fen) {
        return fen ? new Chess(fen).turn() : 'w';
    }

    default_config() {
        return {
            initial_move_delay: 500,
            subsequent_move_delay: 250,
        };
    }

    destroy() {
        this._board.destroy();
    }

    configure(config) {
        config = Object.assign(this._config, config);
        this._config = config;

        if (config.moves) {
            this._moves = config.moves ? config.moves.split(" ") : [];
        }

        this._game = config.fen ? new Chess(config.fen) : new Chess();

        // Save computer and player colors.
        if (this._moves[0]) {
            let first_move_origin_square = this._moves[0].slice(0, 2);
            this._computer_color = this._game.get(first_move_origin_square).color;
            this._player_color = this._computer_color == 'w' ? 'b' : 'w';
        }

        // Set board settings.
        this._board.set({
            premovable: {
                enabled: true,
                events: {
                    set: this._set_premove.bind(this),
                    unset: this._unset_premove.bind(this)
                }
            },
            events: {
                move: this._move.bind(this)
            },
            orientation: this._player_color == 'w' ? 'white' : 'black'
        });

        // Start puzzle.
        this.reset();
    }
    
    get_dests(square) {
        let res = this._game.moves({ square })
            .map(move => move.replace("+", "").replace("#", "").slice(-2));
        return res;
    }

    reset() {
        this._premove = null;
        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;

        this._failed = false;

        this._game = this._config.fen ? new Chess(this._config.fen) : new Chess();
        this._board_states = [this.initial_board_state()];
        this.sync_board();

        this._remaining_moves = this._moves.slice();

        if (this._config.locked || this._remaining_moves.length == 0) {
            this._board.set({
                movable: {
                    color: 'none',
                }
            });
        }

        if (this._remaining_moves.length > 0) {
            setTimeout(this._make_computer_move.bind(this), this._config.first_move_delay);
        }
    }

    is_puzzle_loaded() {
        return this._moves.length > 0;
    }

    is_complete() {
        return this._moves.length > 0 && this._remaining_moves.length == 0;
    }

    is_failed() {
        return this._failed;
    }

    is_first_move() {
        return this._remaining_moves.length == this._moves.length - 1
            && !this.is_complete()
            && !this.is_failed();
    }

    // Allow pieces to be moved if we're seeked to the latest move, or if the puzzle is over.
    can_move() {
        return this.is_complete() || this.is_failed() || this._seek_position == this.seek_max();
    }

    // Allow premoves as long as the puzzle isn't complete or failed, otherwise it doesn't make
    // sense because you're playing both sides anyway.
    can_premove() {
        return !this.is_complete() && !this.is_failed() && this._seek_position == this.seek_max();
    }

    // Sync the board with the game state.
    sync_board() {
        if (this._seek_position < this._board_states.length) {
            let board_state = this._board_states[this._seek_position];

            // Let the player make moves for both sides if the puzzle is over.
            let movable_color;
            if (this.is_complete() || this.is_failed()) {
                movable_color = board_state.side_to_move == 'b' ? 'black' : 'white';
            }
            // Otherwise only allow them to move their own color.
            else {
                movable_color = this._player_color == 'b' ? 'black' : 'white';
            }

            this._board.set({
                selected: null,
                fen: board_state.fen,
                lastMove: board_state.highlight_move,
                turnColor: this._game.turn() == 'w' ? 'white' : 'black',
                check: this._game.isCheck(),
                movable: {
                    showDests: this.can_move(),
                    dests: { get: (square) => { if (this.can_move()) return this.get_dests(square); } },
                    color: movable_color,
                },
                premovable: {
                    enabled: this.can_premove(),
                    events: {
                        set: (orig, dest) => { if (this.can_premove()) this._set_premove(orig, dest); },
                        unset: (orig, dest) => { if (this.can_premove()) this._unset_premove(orig, dest); },
                    }
                },
            });
        }
    }

    color_to_move() {
        return this._game.turn();
    }

    computer_to_move() {
        return this.color_to_move() == this._computer_color;
    }

    player_color() {
        return this._player_color;
    }

    fen() {
        return this._game.fen();
    }

    awaiting_promotion() {
        return this._awaiting_promotion;
    }

    promote(piece) {
        if (!this.awaiting_promotion()) {
            console.error('promote() called but puzzle is not awaiting a promotion selection');
        }

        if (piece != 'q' && piece != 'r' && piece != 'n' && piece != 'b') {
            console.error(`Invalid promotion piece: ${piece}`);
            return;
        }

        // Apply promotion move.
        let orig = this._awaiting_promotion_move[0];
        let dest = this._awaiting_promotion_move[1];
        console.log(`Applying promotion move ${orig}${dest}${piece}`);
        this._make_player_move(orig, dest, piece);

        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;
    }

    cancel_promotion() {
        if (!this._awaiting_promotion) {
            console.error('cancel_promotion() called but puzzle is not awaiting a promotion selection');
        }

        // Reset board back to before the player made a move. The move hasn't been applied to the
        // game yet so that doesn't need updating.
        this.sync_board();

        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;
    }

    _await_promotion(orig, dest) {
        this._awaiting_promotion = true;
        this._awaiting_promotion_move = [orig, dest];
        if (this._config.on_promote)
            this._config.on_promote();
    }

    _make_computer_move() {
        if (this._game.turn() != this._computer_color) {
            console.warn("_make_computer_move(): called when it's the player's turn");
            return;
        }

        // Get the next computer move.
        let next_move = this._remaining_moves.shift();
        if (next_move) {
            console.log(`Making next computer move ${next_move}`);
            // Get the orig and destination of the move.
            let orig = next_move.slice(0, 2);
            let dest = next_move.slice(2);

            // Update the game.
            this._game.move(next_move);

            // Store board state.
            this.push_board_state([orig, dest], this._player_color);

            // Sync chess board.
            this.sync_board();

            // Call the on_move callback.
            if (this._config.on_move)
                this._config.on_move();

            // Add a timer to apply the next premove, if there is one.
            setTimeout(() => this._board.playPremove(), this._config.subsequent_move_delay);
        }
    }

    // Apply a player move to the game and board, and check whether it was the right one.
    // `promotion` can either be the chosen promotion piece, or unspecified.
    _make_player_move(orig, dest, promotion) {
        if (this._game.turn() != this._player_color) {
            console.warn("_make_player_move(): called when it's the computer's turn");
        }

        console.log(`Making player move ${orig}${dest}${promotion ? promotion : ""}`);

        // Apply move to chess.js, and return if it was an illegal move or otherwise rejected.
        // This shouldn't happen because _move validates it anyway, but it's good to double check.
        try {
            this._game.move({
                from: orig,
                to: dest,
                promotion
            });
        }
        catch (e) {
            console.warn(`Rejecting invalid move ${orig}${dest}${promotion ? promotion : ""}`);
            console.log(e);
            this.sync_board();
            return;
        }

        // Store board state.
        this.push_board_state([orig, dest], this._computer_color);

        // Call on move callback now that we've validated it.
        if (this._config.on_move)
            this._config.on_move();

        // Check if it was the right or wrong move. A special case we need to consider is that for
        // checkmate-in-ones there might be multiple legal moves. This is the only special case
        // listed at https://database.lichess.org/#puzzles (all other moves are "only moves").
        // I originally ignored this until I found the example puzzle ULF41, in which the final
        // move is listed as Qe1# instead of the (imo) more natural Qf1#, although they are both
        // legal moves *and* checkmate.
        let move = orig + dest + (promotion ? promotion : "");

        if (this._game.isCheckmate()) {
            // If it was a checkmate it's definitely the last move.
            this._remaining_moves.length = 0;
            if (this._config.on_success)
                this._config.on_success();
        }
        else if (this._remaining_moves[0] == move) {
            this._remaining_moves.shift();

            // If there are no moves left, the puzzle is complete.
            if (this._remaining_moves.length == 0) {
                if (this._config.on_success)
                    this._config.on_success();
            }
            else {
                if (this._config.on_right_move)
                    this._config.on_right_move();

                setTimeout(this._make_computer_move.bind(this), this._config.subsequent_move_delay);
            }
        }
        else {
            this._failed = true;
            if (this._config.on_wrong_move)
                this._config.on_wrong_move();
        }

        this.sync_board();
    }

    // The callback for a player making a move in chessground. If the selected move was a promotion,
    // we call the promotion callback so the player can be prompted for their selected promotion,
    // otherwise we just apply the move immediately to the game and board.
    _move(orig, dest, _) {
        if (!this.can_move()) {
            this.sync_board();
            return;
        }

        // If the puzzle is over, just allow free movement of pieces as long as they're legal moves.
        if (this.is_complete() || this.is_failed()) {
            try {
                this._game.move({
                    from: orig,
                    to: dest,
                    promotion: 'q',
                });
                this.push_board_state([orig, dest], this._game.turn());
                this.sync_board();
            }
            catch (_) {
                this.sync_board();
            }
            return;
        }

        // Use chess.js to check if the move was legal.
        let result;
        try {
            result = this._game.move({
                from: orig,
                to: dest,
                // Just make all promotions queens for the purposes of checking for legal moves,
                // later if it was legal we'll prompt for the type of promotion.
                promotion: 'q',
            });

            // For now we undo the move if it succeeded, because if it was a promotion, we need to
            // kick it back to the frontend to prompt the user for their desired promotion piece.
            // _make_player_move() actually applies the move later.
            this._game.undo();
        }
        catch (e) {
            console.log(`Rejecting invalid move attempt ${orig}${dest}`);
            this.sync_board();
            return;
        }

        // If the move was a promotion, call the promotion callback and kick it back to the user
        // to decide the promotion piece.
        if (result.promotion) {
            console.log("Move is promotion, prompting for promotion piece");
            this._await_promotion(orig, dest);
            return;
        }

        // Now we're actually done validating / making sure a promotion has been specified and are
        // ready to make the move.
        this._make_player_move(orig, dest);
    }

    _set_premove(orig, dest) {
        this._premove = [orig, dest];
    }

    _unset_premove(orig, dest) {
        this._premove = null;
    }

    _seek_game() {
        let board_state = this._board_states[this._seek_position];
        this._game = new Chess(board_state.fen);
    }

    seek_position() {
        return this._seek_position;
    }

    seek_max() {
        return this._board_states.length - 1;
    }

    seek_start() {
        this._seek_position = 0;
        this._seek_game();
        this.sync_board();

        if (this._config.on_seek)
            this._config.on_seek();
    }

    seek_prev() {
        this._seek_position = Math.max(0, this._seek_position - 1);
        this._seek_game();
        this.sync_board();

        if (this._config.on_seek)
            this._config.on_seek();
    }

    seek_next() {
        this._seek_position = Math.min(this.seek_max(), this._seek_position + 1);
        this._seek_game();
        this.sync_board();

        if (this._config.on_seek)
            this._config.on_seek();
    }

    seek_end() {
        this._seek_position = this.seek_max();
        this._seek_game();
        this.sync_board();

        if (this._config.on_seek)
            this._config.on_seek();
    }
}
