console.log("Better tactics");

import { Chess } from './deps/chess.js';
import { Chessground } from './deps/chessground.min.js';
import {
    init,
    classModule,
    propsModule,
    styleModule,
    eventListenersModule,
    h,
} from './deps/snabbdom.js';

console.log("Initialising snabbdom");
const patch = init([
  // Init patch function with chosen modules
  classModule, // makes it easy to toggle classes
  propsModule, // for setting properties on DOM elements
  styleModule, // handles styling on elements with support for animations
  eventListenersModule, // attaches event listeners
]);


function view(currentDate) {
    return h('div', 'Current date ' + currentDate);
}

let vnode = patch($("#topper-bar")[0], view(new Date()));

setInterval(function() {
    const newVNode = view(new Date());
    vnode = patch(vnode, newVNode);
}, 1000);

const COMPUTER_MOVE_DELAY = 250;

export class PuzzleBoard {
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

        // Save computer and player colors.
        let first_move_origin_square = this._moves[0].slice(0, 2);
        this._computer_color = this._game.get(first_move_origin_square).color;
        this._player_color = this._computer_color == 'w' ? 'b' : 'w';

        // Set board settings.
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
                move: this._move.bind(this)
            },
            orientation: this._player_color == 'w' ? 'white' : 'black'
        });

        // Start puzzle.
        this.reset();
    }
    
    get_dests(square) {
        return this._game.moves({ square })
            .map(move => move.replace("+", "").replace("#", "").slice(-2));
    }

    reset() {
        this._game = new Chess(this._fen);
        this.sync_board();

        this._remaining_moves = this._moves.slice();

        // Make the player's pieces movable again, the 'wrong move' interface disables this.
        this._board.set({
            movable: {
                color: this._player_color == 'w' ? 'white' : 'black'
            }
        });

        setTimeout(this._make_computer_move.bind(this), COMPUTER_MOVE_DELAY);
    }

    // Sync the board with the game state.
    sync_board() {
        this._board.set({
            fen: this._game.fen(),
            lastMove: this._last_move ? this._last_move : null,
            turnColor: this._game.turn() == 'w' ? 'white' : 'black',
            check: this._game.isCheck(),
            movable: {
                dests: { get: this.get_dests.bind(this) }
            },
        });
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
            // Get the orig and destination of the move.
            let orig = next_move.slice(0, 2);
            let dest = next_move.slice(2);

            // Store the last move for highlighting purposes.
            this._last_move = [orig, dest];

            // Update the game and board.
            this._game.move(next_move);
            this.sync_board();

            // Call the on_move callback.
            this._on_move();

            // Add a timer to apply the next premove, if there is one.
            setTimeout(() => this._board.playPremove(), COMPUTER_MOVE_DELAY);
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

        // Store the last move for highlighting purposes.
        this._last_move = [orig, dest];

        // Sync the board to apply any promotions etc. We do this now, after checking if a
        // promotion piece has been specified, as otherwise it defaults to Knight promotion
        // and looks a bit wonky.
        this.sync_board();

        // Call on move callback now that we've validated it.
        this._on_move();

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
            this._on_success();
        }
        else if (this._remaining_moves[0] == move) {
            this._remaining_moves.shift();

            // If there are no moves left, the puzzle is complete.
            if (this._remaining_moves.length == 0) {
                this._on_success();

                // As an extra thing, let the player move the opponent's pieces so they can keep
                // playing out the game if they like.
                this._board.set({
                    movable: {
                        color: this._game.turn() == 'w' ? 'white' : 'black'
                    }
                });
            }
            else {
                this._right_move();
                setTimeout(this._make_computer_move.bind(this), COMPUTER_MOVE_DELAY);
            }
        }
        else {
            this._remaining_moves.length = 0;

            // Disable the move while showing the 'wrong move' interface.
            this._board.set({
                movable: {
                    color: 'none'
                }
            });

            this._wrong_move();
        }
    }

    // The callback for a player making a move in chessground. If the selected move was a promotion,
    // we call the promotion callback so the player can be prompted for their selected promotion,
    // otherwise we just apply the move immediately to the game and board.
    _move(orig, dest, _) {
        // If the puzzle is over, just allow free movement of pieces as long as they're legal moves.
        if (this._remaining_moves.length == 0) {
            try {
                this._game.move({
                    from: orig,
                    to: dest,
                    promotion: 'q'
                });
                this._last_move = [orig, dest];
                this.sync_board();

                // Set the next color as movable.
                this._board.set({
                    movable: {
                        color: this._game.turn() == 'w' ? 'white' : 'black'
                    }
                });
            }
            catch (_) {
                this.sync_board();
            }
            return;
        }

        // Use chess.js to check if the move was legal.
        try {
            let result = this._game.move({
                from: orig,
                to: dest,
                // Just make all promotions queens for the purposes of checking for legal moves,
                // later if it was legal we'll prompt for the type of promotion.
                promotion: 'q'
            });

            // For now we undo the move if it succeeded, because if it was a promotion, we need to
            // kick it back to the frontend to prompt the user for their desired promotion piece.
            // _make_player_move() actually applies the move later.
            this._game.undo();

            // If the move was a promotion, call the promotion callback and kick it back to the user
            // to decide the promotion piece.
            if (result.promotion) {
                console.log("Move is promotion, prompting for promotion piece");
                this._await_promotion(orig, dest);
                return;
            }
        }
        catch (e) {
            console.log(`Rejecting invalid move attempt ${orig}${dest}`);
            this.sync_board();
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
}
