console.log("Better tactics");

import { Chess } from './deps/chess.js';
import { Chessground } from './deps/chessground.min.js';

const COMPUTER_MOVE_DELAY = 250;

export class Puzzle {
    constructor(container, puzzle_id, fen, moves, rating, on_success, on_move, right_move, wrong_move,
        on_promote)
    {
        this._puzzle_id = puzzle_id;
        this._fen = fen;
        this._moves = moves.split(" ");
        this._rating = rating;

        this._on_success = on_success;
        this._on_move = on_move;
        this._right_move = right_move;
        this._wrong_move = wrong_move;
        this._on_promote = on_promote;

        this._premove = null;
        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;

        this._game = new Chess(fen);
        this._board = Chessground(container);

        // Figure out who should move first (the move that sets up the tactic), make the move,
        // and let the player make the next move.
        let first_move_origin_square = this._moves[0].slice(0, 2);

        this._computer_color = this._game.get(first_move_origin_square).color;
        this._player_color = this._computer_color == 'w' ? 'b' : 'w';

        // Start puzzle.
        this.reset();
    }

    reset() {
        this._game = new Chess(this._fen);

        this._remaining_moves = this._moves.slice();

        this._board.set({
            movable: {
                showDests: true,
                color: this._player_color == 'w' ? 'white' : 'black',
            },
            premovable: {
                events: {
                    set: this._set_premove.bind(this),
                    unset: this._unset_premove.bind(this)
                }
            },
            events: {
                move: this._make_player_move.bind(this)
            },
            orientation: this._player_color == 'w' ? 'white' : 'black',
            turnColor: this._computer_color == 'w' ? 'white' : 'black',
            fen: this._fen
        });

        setTimeout(this._make_computer_move.bind(this), COMPUTER_MOVE_DELAY);
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
        this._make_player_move(orig, dest, {}, piece);

        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;
    }

    cancel_promotion() {
        if (!this._awaiting_promotion) {
            console.error('cancel_promotion() called but puzzle is not awaiting a promotion selection');
        }

        // Reset board back to before the player made a move. The move hasn't been applied to the
        // game yet so that doesn't need updating.
        this._board.set({
            fen: this._last_fen,
            lastMove: this._last_move,
            turnColor: this._player_color == 'w' ? 'white' : 'black'
        });

        this._awaiting_promotion = false;
        this._awaiting_promotion_move = null;
    }

    _await_promotion(orig, dest) {
        this._awaiting_promotion = true;
        this._awaiting_promotion_move = [orig, dest];
        this._on_promote();
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
            // Get the source and destination of the move.
            let source = next_move.slice(0, 2);
            let dest = next_move.slice(2);

            // Update the game and board.
            this._game.move(next_move);
            this._board.set({
                fen: this._game.fen(),
                lastMove: [source, dest],
                turnColor: this._player_color == 'w' ? 'white' : 'black',
                check: this._game.isCheck()
            });
            this._board.move(next_move);

            // Store the last move for highlighting purposes.
            this._last_fen = this._game.fen();
            this._last_move = [source, dest];

            // Call the on_move callback.
            this._on_move();

            // Add a timer to apply the next premove, if there is one.
            setTimeout(() => this._board.playPremove(), COMPUTER_MOVE_DELAY);
        }
    }

    _make_player_move(orig, dest, _, promotion) {
        let move = orig + dest;
        let move_is_promotion = this._move_is_promotion(orig, dest);

        // If a promotion has been specified (e.g. by calling this.promote()), add it to the move text.
        if (promotion) {
            move += promotion;
        }

        // If the puzzle is over, reject any moves.
        if (this._remaining_moves.length == 0) {
            this._board.set({
                fen: this._game.fen(),
                lastMove: this._last_move,
                turnColor: this._player_color == 'w' ? 'white' : 'black'
            });
            return;
        }

        // Make the move in chess.js, and if it was an illegal move, reject it.
        try {
            this._game.move(move);

        }
        catch (_) {
            this._board.set({
                fen: this._game.fen(),
                lastMove: this._last_move,
                turnColor: this._player_color == 'w' ? 'white' : 'black',
                check: this._game.isCheck()
            });
            return;
        }

        // If the move is a promotion, we need to prompt the user to find out what piece they're
        // promoting to.
        if (move_is_promotion && !promotion) {
            console.log("Move is promotion, prompting user for promotion piece");

            // Undo the move in chess.js while we wait.
            this._game.undo();

            this._await_promotion(orig, dest);
            return;
        }

        // Update the board state to match the chess.js state. (e.g. change promoted pawns to
        // pieces, and set whether the player to move is in check.
        this._board.set({
            fen: this._game.fen(),
            lastMove: [orig, dest],
            turnColor: this._computer_color == 'w' ? 'white' : 'black',
            check: this._game.isCheck()
        });

        // Call on move callback now that we've validated it.
        this._on_move();

        // Check if it was the right or wrong move. A special case we need to consider is that for
        // checkmate-in-ones there might be multiple legal moves. This is the only special case
        // listed at https://database.lichess.org/#puzzles (all other moves are "only moves").
        // I originally ignored this until I found the example puzzle ULF41, in which the final
        // move is listed as Qe1# instead of the (imo) more natural Qf1#, although they are both
        // legal moves *and* checkmate.
        if (this._game.isCheckmate()) {
            this._remaining_moves.length = 0;
            this._on_success();
        }
        else if (this._remaining_moves[0] == move) {
            this._remaining_moves.shift();

            // If there are no moves left, the puzzle is complete.
            if (this._remaining_moves.length == 0) {
                this._on_success();
            }
            else {
                this._right_move();
                setTimeout(this._make_computer_move.bind(this), COMPUTER_MOVE_DELAY);
            }
        }
        else {
            this._remaining_moves.length = 0;
            this._board.set({
                movable: {
                    color: 'none'
                }
            });
            this._wrong_move();
        }
    }

    _set_premove(orig, dest) {
        this._premove = [orig, dest];
    }

    _unset_premove(orig, dest) {
        this._premove = null;
    }

    _move_is_promotion(orig, dest) {
        // Get the piece to move to check if it's a pawn.
        let piece = this._game.get(orig);
        if (!piece) {
            console.error(`_move_is_promotion() called but no piece is at origin ${orig}`);
            return false;
        }

        // Check if the destination is the backrank for the side moving.
        let dest_is_backrank = (piece.color == 'w' && dest[1] == '8') ||
            (piece.color == 'b' && dest[1] == '1');

        return piece.type == 'p' && dest_is_backrank;
    }
}
