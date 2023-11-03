import {
    init,
    classModule,
    propsModule,
    styleModule,
    datasetModule,
    attributesModule,
    eventListenersModule,
    h,
    VNode,
} from 'snabbdom';
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
    vnode: Element | VNode;
    config: any;
    boards: any[];
    data: any;
    data_request_error: string = null;
    container_tag: string = 'div#puzzle-history';

    constructor(element, config) {
        this.vnode = element;
        this.config = {};
        this.boards = [];

        this.configure(config ? config : {});
    }

    configure(config) {
        console.log(config);
        this.config = Object.assign(this.config, config);
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
        return h(this.container_tag, [
            this.error(),
            this.pagination(),
            this.puzzles(),
            this.pagination(),
        ]);
    }

    error_view(err) {
        let error_text = "";
        if (err && err.message) {
            error_text = err.message;
            console.error(err);
        }

        return h(this.container_tag + '.error.fatal', `Error when building view: ${error_text}`);
    }

    error() {
        if (this.data_request_error) {
            return h('div.error.fatal', this.data_request_error);
        }
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
                        attrs: { href: `/tactics/by_id/${puzzle.puzzle_id}` }
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
                                    h('a', { props: { href: `/tactics/by_id/${puzzle.puzzle_id}` } },
                                        puzzle.puzzle_id),
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
                    this.data_request_error = `Failed to get puzzle history: ${err.responseJSON.error}`;
                    this.config.loading = false;
                    this.render();

                    console.error(this.data_request_error);
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

        let index = 0;
        let boards = this.boards;
        for (const container of document.getElementsByClassName('puzzle-history-board')) {
            if (container instanceof HTMLElement) {
                let puzzle = puzzles[container.dataset.id];

                if (!boards[index]) {
                    boards[index] = new PuzzleBoard(container, {});
                }
                boards[index].configure(Object.assign({
                    locked: true,
                }, puzzle));
            }
            index += 1;
        }
    }
}
