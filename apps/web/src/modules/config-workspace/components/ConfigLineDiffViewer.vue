<template>
  <div class="config-line-diff-viewer">
    <div class="config-line-diff-viewer__header">
      <div class="config-line-diff-viewer__title">
        {{ beforeTitle }}
      </div>
      <div class="config-line-diff-viewer__title">
        {{ afterTitle }}
      </div>
    </div>

    <div class="config-line-diff-viewer__body">
      <div
        v-for="(row, index) in rows"
        :key="index"
        class="config-line-diff-viewer__row"
      >
        <div
          class="config-line-diff-viewer__cell"
          :class="lineClass(row.beforeType)"
        >
          <span class="config-line-diff-viewer__line-number">
            {{ row.beforeNumber ?? "" }}
          </span>
          <span class="config-line-diff-viewer__marker">
            {{ lineMarker(row.beforeType) }}
          </span>
          <code class="config-line-diff-viewer__code">
            {{ row.beforeLine ?? "" }}
          </code>
        </div>

        <div
          class="config-line-diff-viewer__cell"
          :class="lineClass(row.afterType)"
        >
          <span class="config-line-diff-viewer__line-number">
            {{ row.afterNumber ?? "" }}
          </span>
          <span class="config-line-diff-viewer__marker">
            {{ lineMarker(row.afterType) }}
          </span>
          <code class="config-line-diff-viewer__code">
            {{ row.afterLine ?? "" }}
          </code>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

type DiffLineType = "unchanged" | "added" | "removed" | "empty";

interface DiffRow {
  beforeLine: string | null;
  afterLine: string | null;
  beforeNumber: number | null;
  afterNumber: number | null;
  beforeType: DiffLineType;
  afterType: DiffLineType;
}

const props = defineProps<{
  beforeContent: string;
  afterContent: string;
  beforeTitle: string;
  afterTitle: string;
}>();

const rows = computed(() =>
  buildLineDiffRows(
    splitLines(props.beforeContent),
    splitLines(props.afterContent),
  ),
);

function splitLines(content: string): string[] {
  const lines = content.split(/\r?\n/);
  if (lines.length > 1 && lines[lines.length - 1] === "") {
    return lines.slice(0, -1);
  }
  return lines;
}

function buildLineDiffRows(
  beforeLines: string[],
  afterLines: string[],
): DiffRow[] {
  const lcs = buildLcsTable(beforeLines, afterLines);
  const diffRows: DiffRow[] = [];
  let beforeIndex = 0;
  let afterIndex = 0;

  while (beforeIndex < beforeLines.length || afterIndex < afterLines.length) {
    if (
      beforeIndex < beforeLines.length &&
      afterIndex < afterLines.length &&
      beforeLines[beforeIndex] === afterLines[afterIndex]
    ) {
      diffRows.push({
        beforeLine: beforeLines[beforeIndex],
        afterLine: afterLines[afterIndex],
        beforeNumber: beforeIndex + 1,
        afterNumber: afterIndex + 1,
        beforeType: "unchanged",
        afterType: "unchanged",
      });
      beforeIndex += 1;
      afterIndex += 1;
      continue;
    }

    if (
      afterIndex < afterLines.length &&
      (beforeIndex === beforeLines.length ||
        lcs[beforeIndex][afterIndex + 1] >= lcs[beforeIndex + 1][afterIndex])
    ) {
      diffRows.push({
        beforeLine: null,
        afterLine: afterLines[afterIndex],
        beforeNumber: null,
        afterNumber: afterIndex + 1,
        beforeType: "empty",
        afterType: "added",
      });
      afterIndex += 1;
      continue;
    }

    diffRows.push({
      beforeLine: beforeLines[beforeIndex],
      afterLine: null,
      beforeNumber: beforeIndex + 1,
      afterNumber: null,
      beforeType: "removed",
      afterType: "empty",
    });
    beforeIndex += 1;
  }

  return diffRows;
}

function buildLcsTable(beforeLines: string[], afterLines: string[]) {
  const table = Array.from({ length: beforeLines.length + 1 }, () =>
    Array.from({ length: afterLines.length + 1 }, () => 0),
  );

  for (let i = beforeLines.length - 1; i >= 0; i -= 1) {
    for (let j = afterLines.length - 1; j >= 0; j -= 1) {
      table[i][j] =
        beforeLines[i] === afterLines[j]
          ? table[i + 1][j + 1] + 1
          : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  return table;
}

function lineClass(type: DiffLineType) {
  return {
    "is-added": type === "added",
    "is-removed": type === "removed",
    "is-empty": type === "empty",
  };
}

function lineMarker(type: DiffLineType) {
  switch (type) {
    case "added":
      return "+";
    case "removed":
      return "-";
    default:
      return "";
  }
}
</script>

<style scoped>
.config-line-diff-viewer {
  border: 1px solid var(--el-border-color, #dcdfe6);
  border-radius: 4px;
  background: var(--el-bg-color, #ffffff);
  overflow: hidden;
}

.config-line-diff-viewer__header,
.config-line-diff-viewer__row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
}

.config-line-diff-viewer__header {
  border-bottom: 1px solid var(--el-border-color-lighter, #ebeef5);
  background: var(--el-fill-color-lighter, #fafafa);
}

.config-line-diff-viewer__title {
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-regular);
}

.config-line-diff-viewer__title + .config-line-diff-viewer__title {
  border-left: 1px solid var(--el-border-color-lighter, #ebeef5);
}

.config-line-diff-viewer__body {
  max-height: 640px;
  overflow: auto;
}

.config-line-diff-viewer__cell {
  display: grid;
  grid-template-columns: 48px 24px minmax(0, 1fr);
  min-width: 0;
  border-bottom: 1px solid var(--el-border-color-extra-light, #f2f6fc);
  background: var(--el-bg-color, #ffffff);
}

.config-line-diff-viewer__cell + .config-line-diff-viewer__cell {
  border-left: 1px solid var(--el-border-color-lighter, #ebeef5);
}

.config-line-diff-viewer__cell.is-added {
  background: #f0f9eb;
}

.config-line-diff-viewer__cell.is-removed {
  background: #fef0f0;
}

.config-line-diff-viewer__cell.is-empty {
  background: var(--el-fill-color-extra-light, #fafcff);
}

.config-line-diff-viewer__line-number,
.config-line-diff-viewer__marker,
.config-line-diff-viewer__code {
  min-height: 24px;
  line-height: 24px;
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono",
    "Courier New", monospace;
  font-size: 13px;
}

.config-line-diff-viewer__line-number {
  padding: 0 8px;
  color: var(--el-text-color-placeholder);
  text-align: right;
  user-select: none;
}

.config-line-diff-viewer__marker {
  color: var(--el-text-color-secondary);
  text-align: center;
  user-select: none;
}

.config-line-diff-viewer__code {
  padding: 0 12px 0 0;
  color: var(--el-text-color-primary);
  white-space: pre;
  overflow: visible;
}

@media (max-width: 768px) {
  .config-line-diff-viewer__header,
  .config-line-diff-viewer__row {
    grid-template-columns: minmax(0, 1fr);
  }

  .config-line-diff-viewer__title + .config-line-diff-viewer__title,
  .config-line-diff-viewer__cell + .config-line-diff-viewer__cell {
    border-left: 0;
  }
}
</style>
