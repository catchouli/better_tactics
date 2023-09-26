console.log("Better tactics");

import { Chess } from './deps/chess.js';
import { Chessground } from './deps/chessground.min.js';

const COMPUTER_MOVE_DELAY = 200;

export class Puzzle {
    constructor(container, puzzle_id, fen, moves, rating, on_success, on_failure) {
        this._puzzle_id = puzzle_id;
        this._fen = fen;
        this._moves = moves.split(" ");
        this._rating = rating;
        this._on_success = on_success;
        this._on_failure = on_failure;

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
                enabled: false,
            },
            events: {
                move: this._move.bind(this)
            },
            orientation: this._player_color == 'w' ? 'white' : 'black',
            turnColor: this._computer_color == 'w' ? 'white' : 'black',
            fen: this._fen
        });

        setTimeout(this._make_next_move.bind(this), COMPUTER_MOVE_DELAY);
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

            this._last_move = [source, dest];

            this._game.move(next_move);
            this._board.set({
                fen: this._game.fen(),
                lastMove: this._last_move,
                turnColor: this._player_color == 'w' ? 'white' : 'black'
            });
            this._board.move(next_move);
        }
    }

    _move(orig, dest, _) {
        let move = orig + dest;

        // If the puzzle is over, reject any moves.
        if (this._remaining_moves.length == 0) {
            this._board.set({
                fen: this._game.fen(),
                lastMove: this._last_move,
                turnColor: this._player_color == 'w' ? 'white' : 'black'
            });
        }

        // Make the move in chess.js, and if it was an illegal move, reject it.
        try {
            this._game.move(move);
        }
        catch (_) {
            this._board.set({
                fen: this._game.fen(),
                lastMove: this._last_move,
                turnColor: this._player_color == 'w' ? 'white' : 'black'
            });
            return;
        }

        if (this._remaining_moves[0] == move) {
            console.log('Right move!');
            this._remaining_moves.shift();

            // If there are no moves left, the puzzle is complete.
            if (this._remaining_moves.length == 0) {
                this._on_success();
            }
            else {
                setTimeout(this._make_next_move.bind(this), COMPUTER_MOVE_DELAY);
            }
        }
        else {
            this._remaining_moves.length = 0;
            this._board.set({
                movable: {
                    color: 'none'
                }
            });
            this._on_failure();
        }
    }
}
