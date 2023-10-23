import {
    init,
    classModule,
    propsModule,
    styleModule,
    datasetModule,
    attributesModule,
    eventListenersModule,
    h,
} from '../deps/snabbdom.js';
import { PuzzleBoard } from './puzzle-board.js';

const patch = init([
    classModule,
    propsModule,
    styleModule,
    attributesModule,
    eventListenersModule,
    datasetModule,
]);

// Puzzle history.
export class PuzzleHistory {
    constructor(element, config) {
        this.vnode = element;
        this.config = {};
        this.boards = [];
        this.data_request_error = null;
        this.container_tag = 'div#puzzle-history';

        this.configure(config ? config : {});
    }

    configure(config) {
        console.log(config);
        this.config = Object.assign(config, this.config);
        this.render();
        this.request_data();
    }

    render() {
        try {
            this.vnode = patch(this.vnode, this.view());
            this.create_boards();
        }
        catch (err) {
            this.vnode = patch(this.vnode, this.error_view(err));
        }
    }

    view() {
        if (this.data_request_error) {
            return this.error_view(this.data_request_error);
        }

        return h(this.container_tag, [
            this.pagination(),
            this.puzzles(),
            this.pagination(),
        ]);
    }

    error_view(err) {
        let error_text = "";
        if (err && err.message) {
            error_text = err.message;
        }
        else {
            error_text = err;
        }

        return h(this.container_tag + '.error.fatal', [
            h('p', `Error when building view: ${error_text}`),
        ]);
    }

    pagination() {
        if (!this.data) {
            return;
        }

        if (!this.data.puzzles || this.data.puzzles.length == 0 || !this.data.num_pages || !this.config.page)
        {
            return;
        }

        return h('div.pagination', [
            h('div', [
                h('a', {
                    attrs: { disabled: this.config.page <= 1 },
                    on: { click: this.prev_page.bind(this) },
                }, 'Previous')
            ]),
            h('div', `Page ${this.config.page} of ${this.data.num_pages}`),
            h('div', [
                h('a', {
                    attrs: { disabled: this.config.page >= this.data.num_pages },
                    on: { click: this.next_page.bind(this) },
                }, 'Next')
            ]),
        ]);
    }

    puzzles() {
        if (!this.data) {
            return h('p', "Loading puzzles...");
        }

        if (!this.data.puzzles || this.data.puzzles.length == 0) {
            return h('p', `No such page ${this.config.page}`);
        }

        let puzzles = this.data.puzzles.map(item => {
            let puzzle = item.puzzle;
            return [
                h('div.puzzle-history-board-container', [
                    h('a', {
                        attrs: { href: `/tactics/by_id/lichess/${puzzle.source_id}` }
                    },
                    [
                        h('div.puzzle-history-board', {
                            dataset: { id: puzzle.puzzle_id },
                        })
                    ]),
                ]),
                h('div.puzzle-history-info', [
                    h('table.stats', [
                        h('tbody', [
                            h('tr', [
                                h('th', 'Lichess puzzle'),
                                h('td', [
                                    h('a', { props: { href: `/tactics/by_id/lichess/${puzzle.source_id}` } },
                                        puzzle.source_id),
                                ]),
                            ]),
                            this.difficulty_row(item),
                        ]),
                    ]),
                ]),
            ];
        }).flat();

        return h('div#puzzle-history-container', puzzles);
    }

    difficulty_row(item) {
        let difficulty_text = item.skipped ? 'Skipped' : this.difficulty_to_string(item.difficulty);
        if (difficulty_text) {
            return h('tr', [
                h('th', 'Difficulty'),
                h('td', difficulty_text),
            ]);
        }
    }

    difficulty_to_string(difficulty) {
        if (difficulty == 0)
            return "Again";
        else if (difficulty == 1)
            return "Hard";
        else if (difficulty == 2)
            return "Good";
        else if (difficulty == 3)
            return "Easy";
    }

    request_data() {
        if (typeof this.config.request_data === "function") {
            console.log(`Puzzle history: Calling request_data`);

            this.data_request_error = null;
            this.config.loading = true;
            this.render();

            this.config.request_data(this.config)
                .then(data => {
                    this.received_data(data);
                    this.data = data;
                    this.config.loading = false;
                    this.render();
                })
                .catch(err => {
                    console.error(err);
                    let error = err.responseText;
                    console.log(error);
                    this.data_request_error = `Failed to get puzzle history ${err.responseText}`;
                    this.config.loading = false;
                    this.render();
                });
        }
    }

    received_data(data) {
        // Get rid of unused boards, otherwise they won't be attached to a dom element anymore
        // but we have no way of knowing it.
        let puzzle_count = data.puzzles ? data.puzzles.length : 0;
        this.boards.length = puzzle_count;
    }

    prev_page() {
        this.config.page = Math.max(1, this.config.page - 1);
        this.request_data();
    }

    next_page() {
        this.config.page = Math.min(this.data.num_pages, this.config.page + 1);
        this.request_data();
    }

    create_boards() {
        if (!this.data || !this.data.puzzles) {
            return;
        }

        let puzzles = {};
        this.data.puzzles.map(item => {
            puzzles[item.puzzle.puzzle_id] = item.puzzle;
        });

        let boards = this.boards;
        $('div.puzzle-history-board').each(function(index) {
            let puzzle = puzzles[this.dataset.id];
            if (!boards[index]) {
                boards[index] = new PuzzleBoard(this);
            }
            boards[index].configure(Object.assign({
                locked: true,
            }, puzzle));
        });
    }
}
