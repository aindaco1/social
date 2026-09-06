<script setup>
import {
    CategoryScale,
    Chart,
    Filler,
    LinearScale,
    LineController,
    LineElement,
    PointElement,
    Tooltip,
} from 'chart.js';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { appearance } from '../appearance.js';
import { observedNumber, formatObservation } from '../reporting.js';

Chart.register(CategoryScale, LinearScale, LineController, LineElement, PointElement, Tooltip, Filler);

const props = defineProps({
    points: {
        type: Array,
        required: true,
    },
    activeIndex: {
        type: [Number, null],
        default: null,
    },
});

const emit = defineEmits(['select']);

const canvas = ref(null);
let chart = null;
const { resolvedTheme } = appearance;

const chartColors = () => {
    const styles = getComputedStyle(document.documentElement);
    const token = (name) => styles.getPropertyValue(`--dw-${name}`).trim();
    return {
        line: token('chart-line'),
        fill: token('chart-fill'),
        ink: token('ink'),
        muted: token('ink-muted'),
        grid: token('border-soft'),
        surface: token('surface-base'),
    };
};

const isActivePoint = (index) => props.activeIndex !== null && Number(props.activeIndex) === index;

const formatNumber = (value) => {
    return formatObservation(value);
};

const selectPoint = (index) => {
    emit('select', index);
};

const chartData = () => {
    const colors = chartColors();
    return {
        labels: props.points.map((point) => point.label),
        datasets: [
            {
                label: 'Followers',
                type: 'line',
                data: props.points.map((point) => observedNumber(point.value)),
                spanGaps: false,
                borderColor: colors.line,
                pointBackgroundColor: colors.line,
                pointBorderColor: colors.line,
                backgroundColor: colors.fill,
                borderWidth: 2,
                pointRadius: 4,
                pointHoverRadius: 5,
                tension: 0.28,
                fill: true,
            },
        ],
    };
};

const updateActiveElements = () => {
    if (!chart) {
        return;
    }

    const index = props.activeIndex === null ? -1 : Number(props.activeIndex);
    if (!Number.isInteger(index) || !props.points[index] || observedNumber(props.points[index].value) === null) {
        chart.setActiveElements([]);
        chart.tooltip?.setActiveElements([], { x: 0, y: 0 });
        chart.update();
        return;
    }

    const point = chart.getDatasetMeta(0).data[index];
    const position = point ? { x: point.x, y: point.y } : { x: 0, y: 0 };

    chart.setActiveElements([{ datasetIndex: 0, index }]);
    chart.tooltip?.setActiveElements([{ datasetIndex: 0, index }], position);
    chart.update();
};

const createChart = () => {
    if (!canvas.value) {
        return;
    }

    const colors = chartColors();
    chart = new Chart(canvas.value, {
        type: 'line',
        data: chartData(),
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: {
                    display: false,
                },
                tooltip: {
                    backgroundColor: colors.surface,
                    titleColor: colors.ink,
                    bodyColor: colors.ink,
                    borderColor: colors.grid,
                    borderWidth: 1,
                    callbacks: {
                        label: (context) => `Followers: ${context.formattedValue}`,
                    },
                },
            },
            scales: {
                x: {
                    grid: {
                        display: false,
                    },
                    ticks: {
                        color: colors.muted,
                        maxRotation: 0,
                        autoSkip: true,
                        maxTicksLimit: 8,
                    },
                },
                y: {
                    beginAtZero: true,
                    grid: {
                        color: colors.grid,
                    },
                    ticks: {
                        color: colors.muted,
                        precision: 0,
                    },
                },
            },
            onClick: (_event, elements) => {
                if (elements.length) {
                    selectPoint(elements[0].index);
                }
            },
            onHover: (_event, elements) => {
                if (elements.length) {
                    selectPoint(elements[0].index);
                }
            },
        },
    });
    updateActiveElements();
};

onMounted(createChart);

watch(
    () => props.points,
    () => {
        if (!chart) {
            createChart();
            return;
        }

        chart.data = chartData();
        chart.update();
        updateActiveElements();
    },
    { deep: true },
);

watch(
    () => props.activeIndex,
    updateActiveElements,
);

watch(resolvedTheme, () => {
    if (!chart) return;
    const colors = chartColors();
    chart.data = chartData();
    chart.options.scales.x.ticks.color = colors.muted;
    chart.options.scales.y.ticks.color = colors.muted;
    chart.options.scales.y.grid.color = colors.grid;
    Object.assign(chart.options.plugins.tooltip, {
        backgroundColor: colors.surface,
        titleColor: colors.ink,
        bodyColor: colors.ink,
        borderColor: colors.grid,
    });
    chart.update('none');
});

onBeforeUnmount(() => {
    chart?.destroy();
    chart = null;
});
</script>

<template>
    <div class="audience-line-chart">
        <div class="audience-line-canvas">
            <canvas ref="canvas" aria-label="Audience followers line chart" role="img"></canvas>
        </div>
        <div class="audience-line-points" aria-label="Audience points">
            <button
                v-for="(point, index) in points"
                :key="`${point.label}-${index}`"
                type="button"
                class="audience-line-point"
                :class="{ 'is-active': isActivePoint(index) }"
                :aria-pressed="isActivePoint(index)"
                @click="selectPoint(index)"
            >
                <span>{{ point.label }}</span>
                <strong>{{ formatNumber(point.value) }}</strong>
            </button>
        </div>
    </div>
</template>
