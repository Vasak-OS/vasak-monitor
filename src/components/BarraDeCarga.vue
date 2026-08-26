<script setup lang="ts">
import { computed } from 'vue';
import { tonoDeCarga } from '@/tools/formato';

const props = defineProps<{ porciento: number; etiqueta?: string }>();

const ancho = computed(() => `${Math.min(100, Math.max(0, props.porciento))}%`);
const tono = computed(() => tonoDeCarga(props.porciento));
const color = computed(() =>
	tono.value === 'critico'
		? 'bg-status-error'
		: tono.value === 'atencion'
			? 'bg-status-warning'
			: 'bg-primary'
);
</script>

<template>
	<div class="flex flex-col gap-1">
		<div v-if="etiqueta" class="flex justify-between text-tx-muted text-xs">
			<span>{{ etiqueta }}</span>
		</div>
		<div class="h-2 w-full overflow-hidden rounded-full bg-ui-surface">
			<div class="h-full transition-all duration-300" :class="color" :style="{ width: ancho }"></div>
		</div>
	</div>
</template>
