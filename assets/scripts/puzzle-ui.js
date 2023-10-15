import {
    init,
    classModule,
    propsModule,
    styleModule,
    datasetModule,
    eventListenersModule,
    h,
} from '../deps/snabbdom.js';
import { Chess } from '../deps/chess.js';
import { PuzzleBoard } from './puzzle-board.js';

const patch = init([
    classModule,
    propsModule,
    styleModule,
    eventListenersModule,
    datasetModule,
]);

export class PuzzleUi {
    constructor(container, config) {
        this.config = {};
        this._vnode = container;

        this._analysis_fen = null;

        this.configure(config ? config : {});
    }

    configure(config) {
        config = Object.assign(this.config, config);
        this.config = config;

        // If there's a fen, use it to figure out the fen for analysis. Ideally we'd just pass the
        // puzzle fen and first move to the analysis board, but lichess only supports pgn or fen,
        // and not both.
        this._analysis_fen = null;
        if (config.puzzle && config.puzzle.fen && config.puzzle.moves) {
            try {
                let game = new Chess(config.puzzle.fen);
                let first_move = config.puzzle.moves.split(' ')[0];
                game.move(first_move);
                this._analysis_fen = game.fen();
            }
            catch (e) {
                console.error("Error when using chess.js to get initial board state for analysis");
                console.error(e);
            }
        }

        // Store whether it's currently the first try or not, so we know if it's a successful solve or not.
        this.first_try = true;

        // Render the layout and create the board container.
        this.render();

        console.log(config);

        // Create puzzle board.
        this.puzzle = new PuzzleBoard(document.getElementById("board"));
        this.puzzle.configure({
            puzzle_id: config.puzzle ? config.puzzle.id : null,
            fen: config.puzzle ? config.puzzle.fen : null,
            moves: config.puzzle ? config.puzzle.moves : null,
            puzzle_rating: config.puzzle ? config.puzzle.rating : null,
            on_success: this.on_success.bind(this),
            on_move: this.on_move.bind(this),
            on_right_move: this.on_right_move.bind(this),
            on_wrong_move: this.on_wrong_move.bind(this),
            on_promote: this.on_promote.bind(this),
        });
    }

    render() {
        this._vnode = patch(this._vnode, this.view());
    }

    view() {
        return h('div#puzzle-interface', [
            this.topbar(),

            h('div.columns', [
                // The board column on the left.
                h('div.is-two-thirds.column', [
                    h('div#board-container.bt-panel', [
                        // Chessground container.
                        h('div#board.chessground'),

                        // Promotion ui.
                        this.promotion_ui(),
                    ]),
                ]),

                // The sidebar.
                this.sidebar(),
            ])
        ]);
    }

    topbar() {
        let contents = this.topbar_contents();

        if (contents) {
            return h('div#top-bar.columns.bt-panel', contents);
        }
    }

    topbar_contents() {
        if (this.config.query.mode == "Random") {
            if (this.config.puzzle && this.config.query) {
                let min = this.config.query.min_rating ? this.config.query.min_rating : 0;
                let max = this.config.query.max_rating ? this.config.query.max_rating : 0;
                return `Reviewing random puzzle in rating range ${min}-${max}`;
            }
            else {
                return "No puzzles found in category and rating range";
            }
        }
        else if (this.config.query.mode == "Specific") {
            if (this.config.puzzle) {
                let puzzle_id = this.config.puzzle.id;
                let review_count = this.config.card ? this.config.card.review_count : 0;
                let unseen = review_count > 0 ? "" : " (unseen)";
                return `Reviewing specific puzzle ${puzzle_id}${unseen}`;
            }
            else {
                return 'No such puzzle';
            }
        }
        else if (this.config.query.mode == "Review") {
            if (this.config.puzzle) {
                let reviews_left = this.config.stats.reviews_due_now;
                return `Reviewing next puzzle (${reviews_left} reviews left)`;
            }
            else {
                let due = this.config.stats.next_review_due;
                let done_text = this.config.stats.reviews_due_today > 0
                    ? `You are done with reviews for now, the next review is due in ${due}.`
                    : 'You are done with reviews for today!';

                return h('p', [
                    done_text,
                    h('br'),
                    "Perhaps it's time to try some ",
                    h('a', { props: { href: '/tactics/new' } }, 'new puzzles'),
                    '? Or you can return to the ',
                    h('a', { props: { href: '/' } }, 'main page'),
                    ' to see your stats.'
                ]);
            }
        }
    }

    promotion_ui() {
        if (!this.puzzle || !this.puzzle.awaiting_promotion()) {
            return h('div');
        }

        return h('div#promotion-ui',
            { on: {
                click: this.on_promotion_background_clicked.bind(this),
                contextmenu: e => e.preventDefault(),
            } },
            [
                h('div#white.promotion-pieces', [
                    this.promotion_button('q'),
                    this.promotion_button('r'),
                    this.promotion_button('n'),
                    this.promotion_button('b'),
                ]),
            ]
        );
    }

    promotion_button(piece) {
        return h('button.promotion-piece', {
            dataset: { piece: piece, color: this.puzzle.player_color() },
            on: {
                click: e => this.on_promotion_button_clicked(e),
                contextmenu: e => e.preventDefault(),
            },
        });
    }

    sidebar() {
        return h('div.column.sidebar', [
            this.puzzle_info(),
            this.card_stats(),
            this.side_to_move(),
            this.find_the_line(),
            this.wrong_move(),
            this.right_move(),
            this.puzzle_complete(),
            this.reviewing_ahead(),
        ]);
    }

    puzzle_info() {
        if (!this.config.puzzle) {
            return h('div#puzzle-info.bt-panel', 'No puzzle loaded');
        }

        return h('div#puzzle-info.bt-panel', [
            h('table.stats', [
                h('tbody', [
                    h('tr', [
                        h('th', 'Lichess puzzle'),
                        h('td', [
                            h('a', { props: { href: `/tactics/by_id/${this.config.puzzle.id}` } },
                                this.config.puzzle.id),
                        ]),
                    ]),
                    h('tr', [
                        h('th', 'Puzzle rating'),
                        h('td', this.config.puzzle ? this.config.puzzle.rating : null),
                    ]),
                    h('tr', [
                        h('th', 'User rating'),
                        h('td', this.config.user_rating),
                    ]),
                ]),
            ]),
            this.puzzle_themes(),
            this.analysis_link(),
        ]);
    }

    puzzle_themes() {
        if (this.config.puzzle && this.config.puzzle.themes) {
            let themes = this.puzzle && this.puzzle.is_complete() ? this.config.puzzle.themes : "?";
            return h('div#puzzle-themes', [
                h('p', [
                    h('b', 'Themes: '),
                    themes,
                ]),
            ]);
        }
    }

    analysis_link() {
        if (this._analysis_fen) {
            return h('a', { props: {
                target: "_blank",
                href: `https://lichess.org/analysis/standard/${this._analysis_fen}`,
            } }, "Analyse");
        }
    }

    card_stats() {
        if (this.config.card) {
            let ease = this.config.card.ease;
            return h("div#card-details.bt-panel", [
                h("table.stats", [
                    h("tbody", [
                        h("tr", [
                            h("th", "Reviews"),
                            h("td", this.config.card.review_count),
                        ]),
                        h("tr", [
                            h("th", "Ease"),
                            h("td", ease ? ease.toFixed(2) : null),
                        ]),
                        h("tr", [
                            h("th", "Due"),
                            h("td", this.config.card.due),
                        ]),
                    ]),
                ]),
            ]);
        }
    }

    side_to_move() {
        if (this.puzzle && !this.puzzle.is_complete() && !this.puzzle.is_failed()) {
            let turn = this.puzzle.color_to_move();

            let text;
            if (this.puzzle.computer_to_move()) {
                text = "Computer to move";
            }
            else if (turn == 'w') {
                text = "White to move";
            }
            else {
                text = "Black to move";
            }

            return h('div.bt-panel.move-indicator', [
                h('div.to-move-piece', { dataset: { color: turn } }),
                text
            ]);
        }
    }

    find_the_line() {
        if (this.puzzle && this.puzzle.is_first_move()) {
            return h('div#find-the-line.bt-panel.controls-subpanel', 'Find the line!');
        }
    }

    reviewing_ahead() {
        console.log(this.config);
        if (this.config.card && !this.config.card.due_now) {
            return h('div#reviewing-ahead.bt-panel.controls-subpanel',
                "Warning: you are reviewing ahead, which is fine, but it will push this card " +
                "further into the future each time you complete it.");
        }
    }

    wrong_move() {
        if (this.puzzle && this.puzzle.is_failed()) {
            return h('div#wrong-move.bt-panel.controls-subpanel', [
                h('p', 'Wrong move :('),
                h('div.columns.button-container', [
                    h('div.column'),
                    h('div.column', [
                        h('button#try-again.button',
                            { on: { click: () => { this.puzzle.reset(); this.render(); } } },
                            h('p.main-text', "Reset")
                        ),
                    ]),
                    h('div.column'),
                ]),
            ]);
        }
    }

    right_move() {
        if (this.puzzle && !this.puzzle.is_first_move() && !this.puzzle.is_failed() &&
            !this.puzzle.is_complete())
        {
            return h('div#right-move.bt-panel.controls-subpanel',
                "Right move!");
        }
    }

    puzzle_complete() {
        let ui = this;

        if (this.puzzle && this.puzzle.is_complete()) {
            if (this.first_try) {
                return h('div#reviewing-ahead.bt-panel.controls-subpanel', [
                    h('p', 'Puzzle complete'),
                    h('div.columns.button-container', [
                        h('div.column', [
                            h('button#hard.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 1 },
                            }, [
                                h('p.main-text', 'Hard'),
                                h('p.sub-text', this.config.card ? this.config.card.hard_time : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#good.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 2 },
                            }, [
                                h('p.main-text', 'Good'),
                                h('p.sub-text', this.config.card ? this.config.card.good_time : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#easy.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 3 },
                            }, [
                                h('p.main-text', 'Easy'),
                                h('p.sub-text', this.config.card ? this.config.card.easy_time : null),
                            ]),
                        ]),
                    ]),
                    h('p', 'Select the difficulty of the puzzle. The next review will be queued ' +
                        'after the amount of time shown.'),
                ]);
            }
            else {
                return h('div#reviewing-ahead.bt-panel.controls-subpanel', [
                    h('p', 'Puzzle complete (with mistakes)'),
                    h('div.columns.button-container', [
                        h('div.column'),
                        h('div.column', [
                            h('button#again.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 0 },
                            }, [
                                h('p.main-text', 'Again'),
                                h('p.sub-text', this.config.card ? this.config.card.again_time : null),
                            ]),
                        ]),
                        h('div.column'),
                    ]),
                    h('p', [
                        h('a#override-link',
                            { on: { click: () => { this.first_try = true; this.render(); } } },
                            'Submit positive answer anyway'
                        ),
                        ' (for example, because of a mouse slip)'
                    ]),
                ]);
            }
        }
    }

    on_promotion_button_clicked(event) {
        if (this.puzzle.awaiting_promotion()) {
            let piece = event.target.dataset.piece;
            console.info(`Promoting to piece ${piece}`);

            this.puzzle.promote(piece);
            this.render();
        }
        event.stopPropagation();
    }

    on_promotion_background_clicked() {
        // If the user clicks the background, cancel promotion.
        if (this.puzzle.awaiting_promotion()) {
            console.info("Cancelling promotion");
            this.puzzle.cancel_promotion();
            this.render();
        }
        event.stopPropagation();
    }

    on_review_button_clicked(button) {
        let puzzle_id = this.config.puzzle ? this.config.puzzle.id : "puzzle_id";

        let difficulty = button.data.dataset.difficulty;
        console.log(`Reviewing ${puzzle_id} with difficulty ${difficulty}`);

        $('button.review-button').prop('disabled', true);

        $.ajax({
            type: "POST",
            url: "/api/tactics/review",
            data: JSON.stringify({
                id: puzzle_id,
                difficulty: difficulty
            }),
            contentType: 'application/json; charset=utf-8',
        })
        .done(function() {
            console.log("Done, reloading");
            window.location.reload();
        })
        .fail(function() {
            console.error("Failed to submit review");
            $('button.review-button').prop('disabled', false);
        });
    }

    // Callbacks for puzzle module.
    on_success() {
        this.render();
    }

    on_move(side_moved, move) {
        this.render();
    }

    on_right_move() {
        this.render();
    }

    on_wrong_move() {
        this.first_try = false;
        this.render();
    }

    on_promote() {
        console.log("Promotion requested, showing promotion ui");
        this.render();
    }
}
