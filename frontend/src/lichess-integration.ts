import { LichessApiClient } from './lichess-client';
import { PuzzleUi } from './puzzle-ui';

export class LichessRetryPuzzleUi {
    lichess: LichessApiClient = new LichessApiClient();
    puzzle_ui: PuzzleUi;

    constructor(puzzle_ui) {
        this.puzzle_ui = puzzle_ui;
        this.render();
    }

    async init() {
        // Initialise lichess client.
        try {
            await this.lichess.init();
        }
        catch (e) {
            console.log(`Error initialising lichess api client: ${e.toString()}`);

            // If something goes wrong initialising the client, try logging out and back in,
            // in case the persistent state somehow got weird, because otherwise the user has
            // no way of fixing it. If it still doesn't work after this though, there must be
            // an issue somewhere else.
            this.lichess.logout();
            await this.lichess.init();
        }

        // Request data now initialised.
        try {
            await this.request_data();
        }
        catch (e) {
            console.error(e);
        }

        this.render();
    }

    async request_data() {
        // Fetch puzzle activity.
        try {
            console.log("Fetching puzzle activity");
            let response = await this.lichess.fetch('api/puzzle/activity?max=1');
            let data = await response.json();
            console.log("Puzzle data:");
            console.log(data);

            // TODO: refactor the PuzzleUi so it can be reused here, and then make the
            // LichessPuzzleUi extend it, but it's initialised first and then request_data.
            // And then get the puzzle ids from the lichess api in batches, and use our api
            // to check if they're already played, and then use our api to get the puzzle data
            // and set up the puzzle... The fen included from the lichess api seems to be after
            // the first computer move which is a bit annoying and causes the puzzles to be inverted.
            // But we should have them all in our database so we can just have an api endpoint that
            // filters the list to ones we have that the user hasn't seen, and then display them as normal.
            this.puzzle_ui.configure({
                mode: "Specific",
                requested_id: data.puzzle.id,
                on_review: console.log,
                request_data: () => {
                    return Promise.resolve({
                        puzzle: {
                            fen: data.puzzle.fen,
                            moves: data.puzzle.solution.join(" "),
                            rating: data.puzzle.rating,
                            puzzle_id: data.puzzle.id,
                            themes: data.puzzle.themes,
                        },
                    });
                },
                on_skip: console.log,
            });
            this.puzzle_ui.request_data();
        }
        catch (e) {
            throw new Error(`Error fetching puzzle activity: ${e.toString()}`);
        }
    }

    render() {
        console.log('render ui');
    }
}
