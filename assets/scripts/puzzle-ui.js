import {
    init,
    classModule,
    propsModule,
    styleModule,
    attributesModule,
    datasetModule,
    eventListenersModule,
    h,
} from '../deps/snabbdom.js';
import { Chess } from '../deps/chess.js';
import { PuzzleBoard } from './puzzle-board.js';
import { asset_path } from './util.js';

const patch = init([
    classModule,
    propsModule,
    styleModule,
    eventListenersModule,
    attributesModule,
    datasetModule,
]);

const DIFFICULTY_AGAIN = 0;
const DIFFICULTY_HARD = 1;
const DIFFICULTY_GOOD = 2;
const DIFFICULTY_EASY = 3;

export class PuzzleUi {
    constructor(container, config) {
        this.config = {};
        this.vnode = container;

        this.analysis_fen = null;
        this.disable_review_buttons = true;

        // Render once and create the puzzle board.
        this.render();
        
        this.puzzle = new PuzzleBoard(document.getElementById("board"), {
            on_success: this.on_puzzle_board_change.bind(this),
            on_move: this.on_puzzle_board_change.bind(this),
            on_right_move: this.on_puzzle_board_change.bind(this),
            on_wrong_move: () => { this.first_try = false; this.on_puzzle_board_change(); },
            on_seek: this.on_puzzle_board_change.bind(this),
            on_promote: this.render.bind(this),
        });

        this.configure(config ? config : {});
        this.request_data();
    }

    on_puzzle_board_change() {
        // If something on the puzzle changes due to user input (e.g. a new move or seeking),
        // disable the next move hint.
        this.puzzle.set_next_move_highlight('none');

        // Re-render the ui to reflect the updated puzzle state.
        this.render();
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
                this.analysis_fen = game.fen();
            }
            catch (e) {
                console.error("Error when using chess.js to get initial board state for analysis");
                console.error(e);
            }
        }

        if (config.board_config) {
            this.puzzle.configure(config.board_config);
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
            this.vnode = patch(this.vnode, this.view());
        }
        catch (err) {
            let error_text = "";
            if (err && err.message) {
                error_text = err.message;
                console.error(err);
            }

            let error_view = h('div.error.fatal', `Error when building view: ${error_text}`);
            this.vnode = patch(this.vnode, error_view);
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
                    this.disable_review_buttons = false;
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
        return h('div#puzzle-interface', [
            this.topbar(),

            h('div.columns', [
                // The board column on the left.
                h('div.is-two-thirds.column', [
                    h('div#board-container.bt-panel', [
                        // Chessground container.
                        h('div#board.chessground', {
                            on: { wheel: this.on_wheel.bind(this) }
                        }),

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
                        due = this.countdown_duration(this.config.stats.ms_until_due);
                        this.start_review_countdown();
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
        if (!this.puzzle) {
            return h('div');
        }

        return h('div#promotion-ui',
            {
                on: {
                    click: this.on_promotion_background_clicked.bind(this),
                    contextmenu: e => e.preventDefault(),
                },
                style: {
                    visibility: this.puzzle.awaiting_promotion() ? "unset" : "hidden",
                },
            },
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
        if (this.puzzle && this.config.puzzle && !this.config.loading) {
            return h('div.column.sidebar', [
                this.move_controls(),
                this.puzzle_info(),
                this.card_stats(),
                this.side_to_move(),
                this.wrong_move(),
                this.hint_button(),
                this.puzzle_complete(),
                this.reviewing_ahead(),
                this.skip_button(),
                this.dont_repeat_button(),
                this.too_easy_button(),
                this.too_hard_button(),
                this.puzzle_themes(),
            ]);
        }
    }

    puzzle_info() {
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
                        h('td', this.puzzle.is_complete() ? this.config.puzzle.rating : '?'),
                    ]),
                    h('tr', [
                        h('th', 'User rating'),
                        h('td', this.config.stats ? this.config.stats.user_rating.rating : ''),
                    ]),
                ]),
            ]),
            this.analysis_link(),
            this.source_url(),
        ]);
    }

    move_controls() {
        let at_start = this.puzzle.seek_position() == 0;
        let at_end = this.puzzle.seek_position() == this.puzzle.seek_max();

        return h('div#move-controls.bt-panel', [
            h('a', {
                dataset: { action: "start" },
                on: { click: this.seek.bind(this) },
                attrs: { disabled: at_start },
            }, [
                h('svg',
                    h('use', { attrs: { href: asset_path("images/icons/play-backwards.svg#icon") } }),
                ),
            ]),
            h('a', {
                dataset: { action: "prev" },
                on: { click: this.seek.bind(this) },
                attrs: { disabled: at_start },
            }, [
                h('svg',
                    h('use', { attrs: { href: asset_path("images/icons/play-track-prev.svg#icon") } }),
                ),
            ]),
            h('a', {
                dataset: { action: "next" },
                on: { click: this.seek.bind(this) },
                attrs: { disabled: at_end },
            }, [
                h('svg',
                    h('use', { attrs: { href: asset_path("images/icons/play-track-next.svg#icon") } }),
                ),
            ]),
            h('a', {
                dataset: { action: "end" },
                on: { click: this.seek.bind(this) },
                attrs: { disabled: at_end },
            }, [
                h('svg',
                    h('use', { attrs: { href: asset_path("images/icons/play-forwards.svg#icon") } }),
                ),
            ]),
        ]);
    }

    puzzle_themes() {
        if (this.puzzle.is_complete() && this.config.puzzle.themes.length > 0)
        {
            let theme_list = this.config.puzzle.themes
                .map(theme => {
                    // Convert camel case to a sentence.
                    return theme.split(/([A-Z][a-z]+)/)
                        .filter(s => s.length != 0)
                        .map(s => s.toLowerCase())
                        .join(' ');
                })

            return h('div#puzzle-themes.bt-panel', [
                h('p', [
                    h('b', 'Themes: '),
                    theme_list.join(', '),
                ]),
            ]);
        }
    }

    analysis_link() {
        if (this.analysis_fen) {
            return h('a.analysis-link', { props: {
                target: "_blank",
                href: `https://lichess.org/analysis/standard/${this.analysis_fen}`,
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

    human_interval(ms) {
        const MS_IN_MINUTE = 60 * 1000;
        const MS_IN_HOUR = 60 * MS_IN_MINUTE;
        const MS_IN_DAY = 24 * MS_IN_HOUR;
        const MS_IN_YEAR = 365 * MS_IN_DAY;

        let result = '';

        // Push a value onto the duration string with a given unit, automatically skipping 0 values
        // and adding pluralisation.
        let push_part = (value, unit) => {
            if (value != 0) {
                let comma = (result.length > 0 ? ', ' : '');
                let pluralisation = (value != 1 ? 's' : '');
                result += `${comma}${value} ${unit}${pluralisation}`;
            }
        };

        let years = Math.floor(ms / MS_IN_YEAR);
        let days = Math.floor((ms % MS_IN_YEAR) / MS_IN_DAY);
        let hours = Math.floor((ms % MS_IN_DAY) / MS_IN_HOUR);
        let minutes = Math.floor((ms % MS_IN_HOUR) / MS_IN_MINUTE);

        if (years > 0) {
            push_part(years, 'year');
            push_part(days, 'day');
        }
        else if (days > 0) {
            push_part(days, 'day');
            push_part(hours, 'hour');
        }
        else {
            push_part(hours, 'hour');
            push_part(minutes, 'minute');
        }

        return result;
    }

    countdown_duration(ms) {
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
        if (this.puzzle.is_puzzle_loaded() && !this.puzzle.is_complete() && !this.puzzle.is_failed())
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

    skip_button() {
        if (this.config.mode == 'Random' && this.puzzle && this.puzzle.is_first_move()) {
            return h('div#skip-button.bt-panel.controls-subpanel', [
                h('a', { on: { click: this.on_skip_clicked.bind(this) } }, "Skip this puzzle"),
            ]);
        }
    }

    too_hard_button() {
        let complete_with_mistakes = this.puzzle.is_complete() && !this.first_try;
        let failed = this.puzzle.is_failed();
        if (this.config.mode == 'Random' && (failed || complete_with_mistakes)) {
            return h('div#too-hard-button.bt-panel.controls-subpanel', [
                h('a', { on: { click: this.on_too_hard_clicked.bind(this) } }, "Too hard (see easier puzzles)"),
            ]);
        }
    }

    dont_repeat_button() {
        let is_success = this.puzzle.is_complete() && this.first_try;
        if (this.config.mode == 'Random' && is_success) {
            return h('div#dont-repeat.bt-panel.controls-subpanel', [
                h('a', { on: { click: this.on_dont_repeat_clicked.bind(this) } }, "Don't repeat this puzzle"),
            ]);
        }
    }

    too_easy_button() {
        let is_success = this.puzzle.is_complete() && this.first_try;
        if (this.config.mode == 'Random' && is_success) {
            return h('div#too-easy.bt-panel.controls-subpanel', [
                h('a', { on: { click: this.on_too_easy_clicked.bind(this) } }, "Too easy (see harder puzzles)"),
            ]);
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
        if (this.puzzle.is_failed()) {
            return h('div#wrong-move.bt-panel.controls-subpanel', [
                h('p', 'Wrong move'),
                h('div.columns.button-container', [
                    h('div.column'),
                    h('div.column', [
                        h('button#try-again.button',
                            { on: { click: () => { this.puzzle.reset(); this.render(); } } },
                            [
                                h('p.main-text', 'Reset'),
                                h('p.sub-text', 'Try again'),
                            ],
                        ),
                    ]),
                    h('div.column'),
                ]),
            ]);
        }
    }

    puzzle_complete() {
        let ui = this;

        if (this.puzzle.is_complete()) {
            if (this.first_try) {
                let card = this.config.card;
                return h('div#reviewing-ahead.bt-panel.controls-subpanel', [
                    h('p', 'Puzzle complete'),
                    h('div.columns.button-container', [
                        h('div.column', [
                            h('button#hard.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: DIFFICULTY_HARD },
                                attrs: {
                                    disabled: this.disable_review_buttons,
                                    title: card ? this.human_interval(card.next_interval_hard) : null,
                                },
                            }, [
                                h('p.main-text', 'Hard'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_hard) : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#good.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: DIFFICULTY_GOOD },
                                attrs: {
                                    disabled: this.disable_review_buttons,
                                    title: card ? this.human_interval(card.next_interval_good) : null,
                                },
                            }, [
                                h('p.main-text', 'Good'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_good) : null),
                            ]),
                        ]),
                        h('div.column', [
                            h('button#easy.button.review-button', {
                                on: { click: function() { ui.on_review_button_clicked(this); } },
                                dataset: { difficulty: DIFFICULTY_EASY },
                                attrs: {
                                    disabled: this.disable_review_buttons,
                                    title: card ? this.human_interval(card.next_interval_easy) : null,
                                },
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
                                dataset: { difficulty: DIFFICULTY_AGAIN },
                                attrs: {
                                    disabled: this.disable_review_buttons,
                                    title: card ? this.human_interval(card.next_interval_again) : null,
                                },
                            }, [
                                h('p.main-text', 'Again'),
                                h('p.sub-text', card ? this.fuzzy_duration(card.next_interval_again) : null),
                            ]),
                        ]),
                        h('div.column'),
                    ]),
                    h('p', [
                        h('a.puzzle-link-button',
                            { on: { click: () => { this.first_try = true; this.render(); } } },
                            'Submit positive answer anyway'
                        ),
                        ' (for example, because of a mouse slip)'
                    ]),
                ]);
            }
        }
    }

    hint_button() {
        if (this.puzzle && !this.puzzle.is_complete() && !this.puzzle.is_failed()) {
            let button_text;

            let mode = this.puzzle.get_next_move_highlight();
            if (mode == 'move')
                button_text = 'Hide hint';
            else if (mode == 'orig')
                button_text = 'Show hint (move)';
            else
                button_text = 'Show hint';

            let disable_button = this.puzzle.computer_to_move() || this.puzzle.awaiting_promotion();

            return h('div#hint-button.bt-panel.controls-subpanel', [
                h('a.puzzle-link-button', {
                    on: { click: this.on_hint_button_clicked.bind(this) },
                    attrs: { disabled: disable_button },
                }, button_text),
            ]);
        }
    }

    on_hint_button_clicked() {
        // Set first_try to false to show the 'again' button by default if the user needed a hint.
        this.first_try = false;

        // If the move is already highlighted, disable the highlight.
        let cur = this.puzzle.get_next_move_highlight();
        if (cur == 'move') {
            this.puzzle.set_next_move_highlight('none');
        }
        // If the piece to move is already highlighted, show the move itself.
        else if (cur == 'orig') {
            this.puzzle.set_next_move_highlight('move');
        }
        // If the mode is 'none', start by highlighting the piece.
        else {
            this.puzzle.set_next_move_highlight('orig');
        }

        this.render();
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
        if (!this.config.card) {
            console.error("Attempt to review when this.config.card does not exist");
            return;
        }

        let card = this.config.card;
        let puzzle_id = card.id;
        let difficulty = button.data.dataset.difficulty;
        console.log(`Reviewing ${puzzle_id} with difficulty ${difficulty}`);

        if (this.config.on_review) {
            this.disable_review_buttons = true;
            this.render();

            this.config.on_review(card, difficulty)
                .then(() => {
                    console.log("Done, loading next puzzle");
                    this.request_data();
                })
                .catch((e) => {
                    this.config.error = `Failed to submit review: ${e.responseText}`;
                    console.error(this.config.error);
                    this.disable_review_buttons = false;
                    this.render();
                });
        }
    }

    on_skip_clicked(button) {
        if (window.confirm("Are you sure you want to skip this puzzle?")) {
            // Skip the puzzle and request a new one. We don't update the rating as the user
            // hasn't solved it, or indicated that it was too hard.
            this.skip_puzzle(DIFFICULTY_GOOD, false);
        }
    }

    on_too_hard_clicked(button) {
        if (window.confirm("Skip this puzzle and see an easier one?")) {
            // Update rating like the puzzle was failed, but don't add it to spaced repetition.
            this.skip_puzzle(DIFFICULTY_AGAIN, true);
        }
    }

    on_dont_repeat_clicked(button) {
        // 'Don't repeat' is part of the success dialog, so we should update the rating as if
        // they'd completed it.
        this.skip_puzzle(DIFFICULTY_GOOD, true);
    }

    on_too_easy_clicked(button) {
        // Update rating like the puzzle was passed, but don't add it to spaced repetition.
        this.skip_puzzle(DIFFICULTY_EASY, true);
    }

    // Scroll wheel.
    on_wheel(evt) {
        if (evt.deltaY < 0) {
            this.puzzle.seek_prev();
        }
        else {
            this.puzzle.seek_next();
        }
        evt.stopPropagation();
        evt.preventDefault();
    }

    skip_puzzle(difficulty, update_rating) {
        if (this.config.on_skip && this.config.card) {
            this.disable_review_buttons = true;

            this.config.on_skip(this.config.card, difficulty, update_rating)
                .then(() => {
                    console.log("Skipped, loading next puzzle");
                    this.request_data();
                })
                .catch((e) => {
                    this.config.error = `Failed to skip puzzle: ${e.responseText}`;
                    console.error(this.config.error);
                    this.disable_review_buttons = false;
                    this.render();
                });
        }
    }

    start_review_countdown() {
        if (!this.countdown_interval && this.config.stats && this.config.stats.ms_until_due > 0) {
            console.log('Starting countdown');
            this.countdown_interval = setInterval(() => {
                this.config.stats.ms_until_due = Math.max(0, this.config.stats.ms_until_due - 1000);
                this.render();

                if (this.config.stats.ms_until_due <= 0) {
                    console.log('Countdown done');
                    clearInterval(this.countdown_interval);
                    this.countdown_interval = null;
                    this.request_data();
                }
            }, 1000);
        }
    }

    seek(button) {
        let action = button.target.dataset.action;

        if (action == "start") {
            this.puzzle.seek_start();
        }
        else if (action == "prev") {
            this.puzzle.seek_prev();
        }
        else if (action == "next") {
            this.puzzle.seek_next();
        }
        else if (action == "end") {
            this.puzzle.seek_end();
        }
    }
}
