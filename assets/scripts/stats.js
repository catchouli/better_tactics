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

/// The length in days of the review forecast.
const REVIEW_FORECAST_LENGTH_DAYS = 10;

/// The threshold for a user's rating deviation to count their rating as provisional.
const RATING_DEVIATION_PROVISIONAL = 100;

/// The maximum number of rating history data points before the point style is disabled.
const RATING_DATA_POINT_CIRCLE_CUTOFF = 20;

// Set default text color for charts.js.
Chart.defaults.color = "rgb(221, 211, 211)";

// Get data for user stats.
$.ajax(`/api/user/stats`)
    .done(function(data) {
        if (data.user_rating.deviation > RATING_DEVIATION_PROVISIONAL) {
            $("#user_rating").text(`${data.user_rating.rating} (+-${data.user_rating.deviation})`);
        }
        else {
            $("#user_rating").text(data.user_rating.rating);
        }

        $("#card_count").text(data.card_count);
        $("#review_count").text(data.review_count);
        $("#reviews_due_now").text(data.reviews_due_now);
        $("#reviews_due_today").text(data.reviews_due_today);
        $("#next_review_due").text(data.next_review_due);

    })
    .fail(function(err) {
        console.error(`Failed to get user stats: ${err.responseText}`);
    })
    .always(function() {
        $("#stats-panel .bt-loader").css("display", "none");
    });

// Get data for review forecast.
$.ajax(`/api/user/review_forecast/${REVIEW_FORECAST_LENGTH_DAYS}`)
    .done(function(data) {
        new Chart(document.getElementById("review-graph"), {
            type: 'bar',
            data: {
                labels: [...data.keys()].map(v => `${v}d`),
                datasets: [
                    {
                        label: "Reviews due",
                        backgroundColor: "#416c86",
                        data: data,
                    }
                ]
            },
            options: {
                maintainAspectRatio: false,
                plugins: {
                    legend: { display: false },
                },
                scales: {
                    x: {
                        title: {
                            display: true,
                            text: "Days from now"
                        }
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
                        }
                    }
                }
            }
        });
    })
    .fail(function(err) {
        console.error(`Failed to get review forecast: ${err.responseText}`);
    })
    .always(function() {
        $("#review-graph-container .bt-loader").css("display", "none");
    });

// Rating graph.
$.ajax("/api/user/rating_history")
    .done(function(data) {
        new Chart(document.getElementById("rating-graph"), {
            type: 'line',
            data: {
                datasets: [{
                    label: "All themes",
                    data: data.map(v => { return { x: v[0], y: v[1] } }),
                    pointStyle: data.length > RATING_DATA_POINT_CIRCLE_CUTOFF ? false : 'circle',
                    tension: 0.01,
                    borderColor: "#416c86",
                }],
            },
            options: {
                maintainAspectRatio: false,
                plugins: {
                    legend: { display: false },
                },
                scales: {
                    x: {
                        type: 'time',
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
            }
        });
    })
    .fail(function(err) {
        console.error(`Failed to get rating history: ${err.responseText}`);
    })
    .always(function() {
        $("#rating-graph-container .bt-loader").css("display", "none");
    });

// Review score histogram.
function add_review_histogram_labels(dataset) {
    return dataset.map(record => {
        record.label = `${record.puzzle_rating_min}-${record.puzzle_rating_max}`;
        return record;
    });
}

function add_percentages(dataset) {
    let totals = {};

    for (let i = 0; i < dataset.length; ++i) {
        let v = dataset[i];
        if (!totals[v.label])
            totals[v.label] = v.review_count;
        else
            totals[v.label] += v.review_count;
    }

    for (let i = 0; i < dataset.length; ++i) {
        let v = dataset[i];
        v.review_percentage = 100 * v.review_count / totals[v.label];
    }
}

export class ReviewScoreChart {
    constructor(element, config) {
        this._vnode = element;
        this.config = {};
        this.mode = 'total';

        this.configure(config ? config : {});
    }

    configure(config) {
        config = Object.assign(config, this.config);
        this.config = config;

        this.render();

        if (config.data) {
            config.data = add_review_histogram_labels(config.data);
            add_percentages(config.data);
            this.create_chart();
        }
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

    view() {
        return h('div#review-score-graph-container.column.bt-panel.chart-panel.stats-panel', [
            h('h2.title.is-3', 'Review scores'),
            h('div.chart-buttons', this.buttons()),
            h('div.chart-container', [
                h('canvas#review-score-graph'),
            ]),
            this.loader(),
        ]);
    }

    buttons() {
        let buttons = [];

        buttons.push(h(
            this.mode == 'total' ? 'b.chart-button' : 'a.chart-button',
            { on: { click: () => this.set_mode('total') } },
            'total'));
        buttons.push(h(
            this.mode == 'percentage' ? 'b.chart-button' : 'a.chart-button',
            { on: { click: () => this.set_mode('percentage') } },
            'percentage'));

        return buttons;
    }

    loader() {
        if (!this.config.data) {
            return h('div.bt-loader');
        }
    }

    set_mode(mode) {
        this.mode = mode;
        this.render();

        if (mode == 'percentage') {
            this.chart.options.parsing.yAxisKey = 'review_percentage';
            this.chart.options.plugins.tooltip.callbacks.label = this.tooltip_percentage.bind(this);
        }
        else if (mode == 'total') {
            this.chart.options.parsing.yAxisKey = 'review_count';
            this.chart.options.plugins.tooltip.callbacks.label = this.tooltip_total.bind(this);
        }
        this.chart.update();
    }

    tooltip_total(item) {
        return `${item.formattedValue}`;
    }

    tooltip_percentage(item) {
        return `${item.parsed.y.toFixed(1)}`;
    }

    create_chart() {
        let data = this.config.data;
        if (!data) {
            return;
        }

        if (this.chart) {
            this.chart.destroy();
        }

        let again_dataset = data.filter(v => v.difficulty == 0);
        let hard_dataset = data.filter(v => v.difficulty == 1);
        let good_dataset = data.filter(v => v.difficulty == 2);
        let easy_dataset = data.filter(v => v.difficulty == 3);

        let review_histogram_labels = new Set(data.map(v => v.label));

        const review_histogram_data = {
            datasets: [
                {
                    label: 'Again',
                    backgroundColor: "#416c86",
                    data: again_dataset,
                    maxBarThickness: 100,
                },
                {
                    label: 'Hard',
                    backgroundColor: '#B7881A',
                    data: hard_dataset,
                    maxBarThickness: 100,
                },
                {
                    label: 'Good',
                    backgroundColor: '#2E680D',
                    data: good_dataset,
                    maxBarThickness: 100,
                },
                {
                    label: 'Easy',
                    backgroundColor: '#269326',
                    data: easy_dataset,
                    maxBarThickness: 100,
                },
            ],
            labels: [...review_histogram_labels],
        };

        this.chart = new Chart(document.getElementById("review-score-graph"), {
            type: 'bar',
            data: review_histogram_data,
            options: {
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
                        title: {
                            display: true,
                            text: "Reviews",
                        },
                        ticks: {
                            precision: 0,
                        },
                    }
                },
                parsing: {
                    xAxisKey: 'label',
                    yAxisKey: 'review_count',
                },
                plugins: {
                    tooltip: {
                        callbacks: {
                            label: this.tooltip_total,
                        },
                    },
                }
            }
        });
    }
}
