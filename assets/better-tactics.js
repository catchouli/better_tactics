console.log("Better tactics");

import { Chess } from './deps/chess.js';
import { Chessground } from './deps/chessground.min.js';

export class Puzzle {
    constructor(container, puzzle_id, fen, moves, rating) {
        this.puzzle_id = puzzle_id;
        this.fen = fen;
        this.moves = moves.split(" ");
        this.rating = rating;

        this.game = new Chess(fen);

        // Figure out who should move first (the move that sets up the tactic), make the move,
        // and let the player make the next move.
        let first_move_orig = this.moves[0].slice(0, 2);
        let first_move_color = this.game.get(first_move_orig).color == 'w' ? 'white' : 'black';
        let player_color = first_move_color == 'white' ? 'black' : 'white';

        console.log("first move color: " + first_move_color + ", player color: " + player_color);

        this.chessground = Chessground(container, {
            movable: {
                free: true,
                color: player_color,
                showDests: true,
                events: {
                    after: this.after_move.bind(this)
                },
            },
            game: this.game,
            fen: this.fen
        });

        this.make_next_move();
    }

    make_next_move() {
        console.log("making next move");
        console.log(this.game._turn);
        let next_move = this.moves.shift();
        if (next_move) {
            let source = next_move.slice(0, 2);
            let dest = next_move.slice(2);

            console.log("Making computer move:" + next_move + " (" + source  + " to " + dest + ")");
            this.game.move(next_move);
            this.chessground.move(source, dest);
            console.log(this.game._turn);
        }
    }

    after_move(orig, dest, metadata) {
        console.log("after move");
        console.log(this.game._turn);
        if (this.moves.length > 0) {
            let move = orig + dest;
            this.game.move(move);
            console.log(this.game._turn);

            if (this.moves[0] == move) {
                console.log('right move!');
                this.moves.shift();
                this.make_next_move();
            }
            else {
                console.log('wrong :(');
            }
        }
    }
}
