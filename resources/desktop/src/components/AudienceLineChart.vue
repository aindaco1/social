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

const isActivePoint = (index) => Number(props.activeIndex) === index;

const formatNumber = (value) => {
    const number = Number(value) || 0;

    return new Intl.NumberFormat().format(number);
};

const selectPoint = (index) => {
    emit('select', index);
};

const chartData = () => ({
    labels: props.points.map((point) => point.label),
    datasets: [
        {
            label: 'Followers',
            type: 'line',
            data: props.points.map((point) => Number(point.value) || 0),
            borderColor: '#3f3795',
            pointBackgroundColor: '#4f46bb',
            pointBorderColor: '#4f46bb',
            backgroundColor: 'rgba(79, 70, 187, 0.09)',
            borderWidth: 2,
            pointRadius: 4,
            pointHoverRadius: 5,
            tension: 0.28,
            fill: true,
        },
    ],
});

const updateActiveElements = () => {
    if (!chart) {
        return;
    }

    if (props.activeIndex === null || props.activeIndex === undefined) {
        chart.setActiveElements([]);
        chart.tooltip?.setActiveElements([], { x: 0, y: 0 });
        chart.update();
        return;
    }

    const index = Number(props.activeIndex);

    if (!Number.isFinite(index) || !props.points[index]) {
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

    chart = new Chart(canvas.value, {
        type: 'line',
        data: chartData(),
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: {
                    display: false,
                },
                tooltip: {
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
                        color: '#64748b',
                        maxRotation: 0,
                        autoSkip: true,
                        maxTicksLimit: 8,
                    },
                },
                y: {
                    beginAtZero: true,
                    grid: {
                        color: '#edf0f3',
                    },
                    ticks: {
                        color: '#64748b',
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
