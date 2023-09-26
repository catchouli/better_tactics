console.log("Better tactics");

import { Chess } from './deps/chess.js';
import { Chessground } from './deps/chessground.min.js';

export class Puzzle {
    constructor(container, puzzle_id, fen, moves, rating) {
        this._puzzle_id = puzzle_id;
        this._fen = fen;
        this._moves = moves.split(" ");
        this._rating = rating;

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
                // TODO: only accept legal moves.
                free: true,
                showDests: true,
                events: {
                    after: this._after_move.bind(this)
                },
            },
            orientation: this._player_color == 'w' ? 'white' : 'black',
            turnColor: this._computer_color == 'w' ? 'white' : 'black',
            fen: this._fen
        });

        this._make_next_move();
    }

    _make_next_move() {
        if (this._game.turn() != this._computer_color) {
            console.warn("_make_next_move(): called when it's the player's turn");
            return;
        }

        console.log("Making next move");

        let next_move = this._remaining_moves.shift();
        if (next_move) {
            let source = next_move.slice(0, 2);
            let dest = next_move.slice(2);

            this._game.move(next_move);
            this._board.set({
                fen: this._game.fen(),
                lastMove: [source, dest],
                turnColor: this._player_color == 'w' ? 'white' : 'black'
            });
            this._board.move(next_move);
        }
    }

    _after_move(orig, dest, metadata) {
        if (this._remaining_moves.length > 0) {
            // Make the move in chess.js.
            let move = orig + dest;
            this._game.move(move);

            if (this._remaining_moves[0] == move) {
                console.log('Right move!');
                this._remaining_moves.shift();

                // If there are no moves left, the puzzle is complete.
                if (this._remaining_moves.length == 0) {
                    console.log("Puzzle complete!");
                }
                else {
                    this._make_next_move();
                }
            }
            else {
                console.log('Wrong move :(');
            }
        }
    }
}
