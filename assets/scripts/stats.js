import {
    init,
    classModule,
    propsModule,
    styleModule,
    datasetModule,
    eventListenersModule,
    h,
} from '../deps/snabbdom.js';

const patch = init([
    classModule,
    propsModule,
    styleModule,
    eventListenersModule,
    datasetModule,
]);

// Set default text color for charts.js.
Chart.defaults.color = "rgb(221, 211, 211)";

// Get an error message from a value depending on the type.
function error_message_from_value(err) {
    if (typeof err === "string") {
        return err;
    }
    else if (typeof err === "object") {
        if (typeof err.message === "string") {
            return err.message;
        }
        else if (typeof err.error == "string") {
            return err.error;
        }
        else if (typeof err.responseJSON === "object" && typeof err.responseJSON.error === "string") {
            return err.responseJSON.error;
        }
        else {
            return "";
        }
    }
}

// User stats.
export class UserStats {
    constructor(element, config) {
        this.vnode = element;
        this.config = {};
        this.container_tag = 'div#stats-panel.column.bt-panel.stats-panel';
        this.data_request_error = null;

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
        }
        catch (err) {
            this.vnode = patch(this.vnode, this.error_view());
        }
    }

    view() {
        let stats = this.config.data ? this.config.data : {};

        return h(this.container_tag, [
            h('h2.title.is-3', 'Stats'),
            this.error(),
            h('table.stats', [
                h('tbody', [
                    h('tr', [
                        h('th', 'User rating'),
                        h('td', stats.user_rating ? stats.user_rating.rating : null),
                    ]),
                    h('tr', [
                        h('th', 'Unique puzzles done'),
                        h('td', stats.card_count),
                    ]),
                    h('tr', [
                        h('th', 'Puzzle completions'),
                        h('td', stats.review_count),
                    ]),
                    h('tr', [
                        h('th', 'Reviews due'),
                        h('td', stats.reviews_due_now),
                    ]),
                    h('tr', [
                        h('th', 'Reviews left today'),
                        h('td', stats.reviews_due_today),
                    ]),
                    h('tr', [
                        h('th', 'Next review due'),
                        h('td', stats.next_review_due),
                    ]),
                ]),
            ]),
            h('a.button', { props: { href: '/tactics', style: 'margin: 0 0.5rem' } }, 'Review'),
            h('a.button', { props: { href: '/tactics/new' } }, 'Next Puzzle'),
            this.loader(),
        ]);
    }

    error_view() {
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

    loader() {
        if (this.config.loading) {
            return h('div.bt-loader');
        }
    }

    request_data() {
        if (typeof this.config.request_data === "function") {
            console.log(`Stats: Calling request_data`);

            this.data_request_error = null;
            this.config.data = null;
            this.config.loading = true;
            this.render();

            this.config.request_data()
                .then(data => {
                    this.config.data = data;
                    this.config.loading = false;
                    this.render();
                })
                .catch(err => {
                    let error_message = error_message_from_value(err);
                    this.data_request_error = `Failed to get data: ${error_message}`;
                    this.config.loading = false;
                    this.render();

                    console.error(this.data_request_error);
                });
        }
    }
}

// A stats chart.
export class StatsChart {
    constructor(element, config) {
        this.vnode = element;
        this.container_id = element.id;
        this.canvas_id = `${this.container_id}_chart`;
        this.container_tag = `div#${this.container_id}.column.bt-panel.chart-panel.stats-panel`;
        this.config = {};
        this.chart_data = {};
        this.data_request_error = null;

        let chart_modes = this.chart_modes();
        this.chart_mode = this.default_mode();

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
            this.create_chart();
        }
        catch (err) {
            this.vnode = patch(this.vnode, this.error_view(err));
        }
    }

    view() {
        if (this.data_request_error) {
            return error_view(this.data_request_error);
        }

        return h(this.container_tag, [
            h('h2.title.is-3', this.chart_title()),
            h('div.chart-buttons', this.buttons()),
            h('div.chart-container', [
                h(`canvas#${this.canvas_id}`),
            ]),
            this.loader(),
        ]);
    }

    error_view(err) {
        let error_message = error_message_from_value(err);
        if (error_message != "") {
            error_message = `Error when building view: ${error_message}`;
        }
        else {
            error_message = `Error when building view`;
        }

        console.error(err);
        return h(this.container_tag + '.error.fatal', error_message);
    }

    request_data() {
        if (typeof this.config.request_data === "function") {
            console.log(`${this.chart_title()}: Calling request_data`);

            this.data_request_error = null;
            this.config.data = null;
            this.render();

            this.config.request_data()
                .then(data => {
                    this.config.data = data;
                    this.render();
                })
                .catch(err => {
                    this.data_request_error = `Failed to get data: ${err.responseText}`;
                    console.error(`${this.chart_title()}: ${this.data_request_error}`);
                    this.render();
                });
        }
    }

    loader() {
        if (!this.config.data) {
            return h('div.bt-loader');
        }
    }

    buttons() {
        function label(mode) {
            if (typeof mode === "object") {
                return mode.label;
            }
            else {
                return mode;
            }
        }

        return this.chart_modes().map(mode => {
            let inactive = label(this.chart_mode) == label(mode) ? '.inactive' : '';
            return h('a.chart-button' + inactive,
                { on: { click: () => { this.chart_mode = mode; this.render(); } } },
                label(mode)
            );
        });
    }

    chart_title() {
        return '';
    }

    chart_options() {
        return {};
    }

    chart_modes() {
        return [];
    }

    default_mode() {
        let chart_modes = this.chart_modes();
        return chart_modes && chart_modes.length > 0 ? chart_modes[0] : '';
    }

    create_chart() {
        if (!this.config.data) {
            return;
        }

        if (this.chart) {
            this.chart.destroy();
        }

        let canvas = document.getElementById(this.canvas_id);
        this.chart = new Chart(canvas, {
            type: this.chart_type(),
            data: this.config.data,
            options: this.chart_options(),
        });
    }

    add_labels(dataset) {
        dataset.map(record => {
            record.label = `${record.puzzle_rating_min}-${record.puzzle_rating_max}`;
        });
    }

    add_percentages(dataset) {
        let totals = {};

        dataset.map(record => {
            if (!totals[record.label])
                totals[record.label] = record.review_count;
            else
                totals[record.label] += record.review_count;
        });

        dataset.map(record => {
            record.review_percentage = 100 * record.review_count / totals[record.label];
        });
    }
}

export class ReviewForecastChart extends StatsChart {
    chart_title() {
        return 'Review forecast';
    }

    chart_type() {
        return 'bar';
    }

    chart_modes() {
        return [
            { label: '7d', x_max: 7 },
            { label: '30d', x_max: 30 },
            { label: 'all', x_max: 1000 },
        ];
    }

    chart_options() {
        return {
            maintainAspectRatio: false,
            plugins: {
                legend: { display: false },
            },
            scales: {
                x: {
                    max: this.chart_mode.x_max,
                    title: {
                        display: true,
                        text: "Days from now"
                    },
                },
                y: {
                    min: 0,
                    suggestedMax: 25,
                    title: {
                        display: true,
                        text: "Reviews due",
                    },
                    ticks: {
                        precision: 0,
                    },
                }
            },
            parsing: {
                xAxisKey: 'day',
                yAxisKey: 'reviews',
            }
        };
    }
}

export class RatingHistoryChart extends StatsChart {
    chart_title() {
        return 'Rating history';
    }

    chart_type() {
        return 'line';
    }

    chart_modes() {
        return [
            { label: '7d', days: 7 },
            { label: '30d', days: 30 },
            { label: 'all', days: 1000 },
        ];
    }

    default_mode() {
        return this.chart_modes()[2];
    }

    chart_options() {
        let min_date = moment().subtract(this.chart_mode.days, 'days');
        console.log('min date: ' + min_date.format());

        if (this.config.data) {
            let data_start = moment(this.config.data.datasets[0].data[0].x);
            if (min_date < data_start) {
                min_date = data_start;
            }
        }

        return {
            maintainAspectRatio: false,
            plugins: {
                legend: { display: false },
            },
            scales: {
                x: {
                    type: 'time',
                    min: min_date,
                    time: {
                        unit: 'day',
                        displayFormats: {
                            'day': 'MMM DD',
                        },
                    }
                },
                y: {
                    title: {
                        display: true,
                        text: "Overall rating",
                    },
                    ticks: {
                        precision: 0,
                    },
                }
            }
        };
    }
}

export class ReviewScoreChart extends StatsChart {
    chart_title() {
        return 'Review scores';
    }

    chart_type() {
        return 'bar';
    }

    chart_modes() {
        return ['total', 'percentage'];
    }

    chart_options() {
        let y_axis_key;
        let y_axis_max;
        let y_axis_label;

        if (this.chart_mode == "percentage") {
            y_axis_key = "review_percentage";
            y_axis_label = "% of reviews";
            y_axis_max = 100;
        }
        else {
            y_axis_key = "review_count";
            y_axis_label = "Number of reviews";
            y_axis_max = null;
        }

        return {
            maintainAspectRatio: false,
            plugins: {
                legend: { display: true },
            },
            scales: {
                x: {
                    stacked: true,
                    title: {
                        display: true,
                        text: "Puzzle rating",
                    },
                },
                y: {
                    stacked: true,
                    suggestedMax: 10,
                    max: y_axis_max,
                    title: {
                        display: true,
                        text: y_axis_label,
                    },
                    ticks: {
                        precision: 0,
                    },
                }
            },
            parsing: {
                xAxisKey: 'label',
                yAxisKey: y_axis_key,
            },
            plugins: {
                tooltip: {
                    callbacks: {
                        label: item => `${item.raw.review_count} (${item.raw.review_percentage.toFixed(2)}%)`,
                    },
                },
            }
        };
    }
}
