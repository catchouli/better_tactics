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

        this.topbar_vnode = h('div');

        this.topbar_interval = setInterval(() => {
            if (!this.config.puzzle && !this.config.error && this.config.stats &&
                this.config.stats.reviews_due_now == 0 &&
                this.config.stats.reviews_due_today > 0)
            {
                if (typeof this.config.stats.ms_until_due === "number") {
                    this.config.stats.ms_until_due -= 1000;

                    if (!this.loading && this.config.stats.ms_until_due < 0) {
                        this.config.stats.ms_until_due = null;
                        this.request_data();
                    }
                    else {
                        this.topbar_vnode = patch(this.topbar_vnode, this.topbar());
                    }
                }
            }
        }, 1000);

        // Render once and create the puzzle board.
        this.render();
        this.puzzle = new PuzzleBoard(document.getElementById("board"), {
            on_success: this.render.bind(this),
            on_move: this.render.bind(this),
            on_right_move: this.render.bind(this),
            on_wrong_move: () => { this.first_try = false; this.render(); },
            on_promote: this.render.bind(this),
        });

        this.configure(config ? config : {});
        this.request_data();
    }

    configure(config) {
        console.log(config);

        if (config.puzzle && config.puzzle.fen && config.puzzle.moves) {
            let puzzle = config.puzzle;

            // Use the fen figure out the fen for analysis. Ideally we'd just pass the puzzle fen
            // and first move to the analysis board, but lichess only supports pgn or fen, and not
            // both; so we use chess.js to figure out the fen after the first computer move and use
            // that instead.
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

        // If a new puzzle is configured.
        if (config.puzzle) {
            // Create puzzle board.
            this.puzzle.configure({
                puzzle_id: config.puzzle ? config.puzzle.puzzle_id : null,
                fen: config.puzzle ? config.puzzle.fen : null,
                moves: config.puzzle ? config.puzzle.moves : null,
                puzzle_rating: config.puzzle ? config.puzzle.rating : null,
            });
        }

        // Store whether it's currently the first try or not, so we know if it's a successful solve or not.
        this.first_try = true;

        // Re-render the layout.
        this.config = Object.assign(this.config, config);
        this.render();
    }

    render() {
        try {
            this._vnode = patch(this._vnode, this.view());
        }
        catch (err) {
            let error_text = "";
            if (err && err.message) {
                error_text = err.message;
                console.error(err);
            }

            let error_view = h('div.error.fatal', `Error when building view: ${error_text}`);
            this._vnode = patch(this._vnode, error_view);
        }
    }

    request_data() {
        if (typeof this.config.request_data === "function") {
            console.log("Puzzle ui: requesting data");
            this.config.error = null;
            this.config.loading = true;
            this.render();

            this.config.request_data()
                .then(data => {
                    this.configure(data);
                    this.config.loading = false;
                    this.render();
                })
                .catch(err => {
                    let error_message = error_message_from_value(err);
                    this.config.error = `Failed to get data: ${error_message}`;
                    this.config.loading = false;
                    this.render();

                    console.error(this.config.error);
                });
        }
    }

    view() {
        this.topbar_vnode = this.topbar();

        return h('div#puzzle-interface', [
            this.topbar_vnode,

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
        return h('div#top-bar.columns.bt-panel', this.topbar_contents());
    }

    topbar_contents() {
        if (this.config.error) {
            return h('div.error.fatal', [ 'Error: ', this.config.error ]);
        }
        else if (this.config.loading) {
            return 'Loading next puzzle...';
        }
        else if (this.config.mode) {
            let mode = this.config.mode;
            if (mode == "Random") {
                if (this.config.puzzle) {
                    let [min, max] = this.config.rating_range ? this.config.rating_range : [0, 0];
                    return `Reviewing random puzzle in rating range ${min}-${max}`;
                }
                else {
                    return "No puzzles found in category and rating range";
                }
            }
            else if (mode == "Specific") {
                if (this.config.puzzle) {
                    let puzzle_id = this.config.puzzle.puzzle_id;
                    let review_count = this.config.card ? this.config.card.review_count : 0;
                    let unseen = review_count > 0 ? "" : " (unseen)";
                    return `Reviewing specific puzzle ${puzzle_id}${unseen}`;
                }
                else {
                    return 'No such puzzle';
                }
            }
            else if (mode == "Review" && this.config.stats) {
                if (this.config.puzzle) {
                    let reviews_left = this.config.stats.reviews_due_now;
                    return `Reviewing next puzzle (${reviews_left} reviews left)`;
                }
                else {
                    let due = 'unknown';
                    if (typeof this.config.stats.ms_until_due === "number") {
                        due = this.human_duration(this.config.stats.ms_until_due);
                    }

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

        return 'error';
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
        if (this.config.puzzle && !this.config.loading) {
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
    }

    puzzle_info() {
        if (!this.config.puzzle)
            return;

        return h('div#puzzle-info.bt-panel', [
            h('table.stats', [
                h('tbody', [
                    h('tr', [
                        h('th', 'Lichess puzzle'),
                        h('td', [
                            h('a', { props: { href: `/tactics/by_id/${this.config.puzzle.puzzle_id}` } },
                                this.config.puzzle.puzzle_id),
                        ]),
                    ]),
                    h('tr', [
                        h('th', 'Puzzle rating'),
                        h('td', this.config.puzzle.rating),
                    ]),
                    h('tr', [
                        h('th', 'User rating'),
                        h('td', this.config.stats ? this.config.stats.user_rating.rating : ''),
                    ]),
                ]),
            ]),
            this.puzzle_themes(),
            this.analysis_link(),
            this.source_url(),
        ]);
    }

    puzzle_themes() {
        if (this.config.puzzle.themes && this.config.puzzle.themes.length > 0) {
            let themes = this.puzzle && this.puzzle.is_complete()
                ? this.config.puzzle.themes.join(', ') : "?";
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
            return h('a.analysis-link', { props: {
                target: "_blank",
                href: `https://lichess.org/analysis/standard/${this._analysis_fen}`,
            } }, "Analyse");
        }
    }

    source_url() {
        return h('a.analysis-link', { props: {
            target: "_blank",
            href: `https://lichess.org/training/${this.config.puzzle.puzzle_id}`,
        } }, "Source");
    }

    card_stats() {
        if (this.config.card && this.config.card.review_count > 0) {
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
                            h("td",
                                { props: { title: this.human_due_date(this.config.card.due) } },
                                this.fuzzy_due_date(this.config.card.due)
                            ),
                        ]),
                    ]),
                ]),
            ]);
        }
    }

    fuzzy_due_date(dt) {
        if (this.has_time_passed(dt)) {
            return "now";
        }
        else {
            return moment(dt).from(moment()).replace('in ', '');
        }
    }

    human_due_date(dt) {
        return moment(dt).format('YYYY-MM-DD HH:mm:ss');
    }

    fuzzy_duration(ms) {
        return moment.duration(ms, 'ms').humanize();
    }

    human_duration(ms) {
        if (ms < 0)
            return "now";
        else if (ms > 60 * 60 * 1000)
            return moment.utc(ms).format("HH:mm:ss");
        else 
            return moment.utc(ms).format("mm:ss");
    }

    has_time_passed(dt) {
        return moment(dt).isBefore(moment());
    }

    side_to_move() {
        if (this.puzzle && this.puzzle.is_puzzle_loaded() && !this.puzzle.is_complete() &&
            !this.puzzle.is_failed())
        {
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
        if (this.config.puzzle && !this.config.due_today) {
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
        if (this.puzzle && this.puzzle.is_puzzle_loaded() && !this.puzzle.is_first_move() &&
            !this.puzzle.is_failed() && !this.puzzle.is_complete())
        {
            return h('div#right-move.bt-panel.controls-subpanel',
                "Right move!");
        }
    }

    puzzle_complete() {
        let ui = this;

        if (this.puzzle && this.puzzle.is_complete()) {
            if (this.first_try) {
                let card = this.config.card;
                return h('div#reviewing-ahead.bt-panel.controls-subpanel', [
                    h('p', 'Puzzle complete'),
                    h('div.columns.button-container', [
                        h('div.column', [
                            h('button#hard.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 1 },
                            }, [
                                h('p.main-text', 'Hard'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_hard) : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#good.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 2 },
                            }, [
                                h('p.main-text', 'Good'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_good) : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#easy.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: 3 },
                            }, [
                                h('p.main-text', 'Easy'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_easy) : null),
                            ]),
                        ]),
                    ]),
                    h('p', 'Select the difficulty of the puzzle. The next review will be queued ' +
                        'after the amount of time shown.'),
                ]);
            }
            else {
                let card = this.config.card;
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
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_again) : null),
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
        let puzzle_id = this.config.puzzle ? this.config.puzzle.puzzle_id : "puzzle_id";
        let difficulty = button.data.dataset.difficulty;
        console.log(`Reviewing ${puzzle_id} with difficulty ${difficulty}`);

        if (this.config.on_review) {
            this.config.on_review(puzzle_id, difficulty);
        }
    }
}
