<template>
  <div class="release-diff-page page-container">
    <LoadingState v-if="projectLoading" />

    <NotFoundState
      v-else-if="projectError && projectError.status === 404"
      :title="t('project.notFound.title')"
      :subtitle="t('project.notFound.subtitle')"
    />

    <ForbiddenState
      v-else-if="projectError && projectError.status === 403"
      :subtitle="t('project.forbidden.subtitle')"
    />

    <ErrorState
      v-else-if="projectError"
      :title="projectError.message"
      @retry="loadAll"
    />

    <template v-else-if="project">
      <PageHeader
        :title="t('releases.diff.title')"
        :subtitle="t('releases.diff.subtitle', { project: project.name })"
      >
        <template #actions>
          <el-button @click="goBackToDetail">
            {{ t("releases.diff.backToDetail") }}
          </el-button>
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="release-diff-page__section">
        <LoadingState v-if="diffLoading" />

        <NotFoundState
          v-else-if="diffError && diffError.status === 404"
          :title="t('releases.detail.notFound')"
          :subtitle="t('releases.detail.notFoundHint')"
        />

        <ForbiddenState
          v-else-if="diffError && diffError.status === 403"
          :subtitle="t('project.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="diffError"
          :title="t('releases.diff.loadError')"
          :subtitle="getErrorMessage(diffError.code, diffError.message)"
          @retry="loadDiff"
        />

        <template v-else-if="diff">
          <!-- Diff meta -->
          <el-descriptions :column="2" border class="release-diff-page__meta">
            <el-descriptions-item :label="t('releases.diff.targetRevision')">
              <span class="release-diff-page__code">
                {{ diff.release.revision }}
              </span>
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.diff.baseRevision')">
              <span v-if="diff.base_release" class="release-diff-page__code">
                {{ diff.base_release.revision }}
              </span>
              <span v-else>{{ t("releases.diff.firstRelease") }}</span>
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.diff.summary')" :span="2">
              <template v-if="diff.diff_summary.is_initial">
                {{ t("releases.diff.firstRelease") }}
              </template>
              <template v-else-if="!diff.diff_summary.has_changes">
                {{ t("releases.diff.noChanges") }}
              </template>
              <template v-else>
                +{{ diff.diff_summary.added_lines }} / -{{
                  diff.diff_summary.removed_lines
                }}
              </template>
            </el-descriptions-item>
          </el-descriptions>

          <!-- Redacted hints -->
          <el-alert
            v-if="diff.before_redacted || diff.after_redacted"
            type="warning"
            :closable="false"
            show-icon
            class="release-diff-page__alert"
          >
            {{ t("releases.diff.redactedHint") }}
          </el-alert>

          <!-- First release hint -->
          <el-alert
            v-if="diff.diff_summary.is_initial"
            type="info"
            :closable="false"
            show-icon
            class="release-diff-page__alert"
          >
            {{ t("releases.diff.firstReleaseHint") }}
          </el-alert>

          <ConfigLineDiffViewer
            :before-content="diff.before_content ?? ''"
            :after-content="diff.after_content"
            :before-title="baseContentTitle"
            :after-title="targetContentTitle"
            class="release-diff-page__content"
          />
        </template>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ConfigLineDiffViewer from "@/modules/config-workspace/components/ConfigLineDiffViewer.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as releasesApi from "@/api/releases";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import type { ReleaseDiffResponse } from "@/api/types/release";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const router = useRouter();
const { t } = useI18nText();

const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));
const releaseId = computed(() => Number(route.params.releaseId));

const diff = ref<ReleaseDiffResponse | null>(null);
const diffLoading = ref(false);
const diffError = ref<ApiRequestError | null>(null);
let diffLoadSeq = 0;

const baseContentTitle = computed(() => {
  if (!diff.value?.base_release) {
    return t("releases.diff.noPreviousContent");
  }

  return `${t("releases.diff.base")} (${diff.value.base_release.revision})`;
});

const targetContentTitle = computed(() => {
  if (!diff.value) {
    return t("releases.diff.target");
  }

  return `${t("releases.diff.target")} (${diff.value.release.revision})`;
});

async function loadDiff() {
  const seq = ++diffLoadSeq;
  diffLoading.value = true;
  diffError.value = null;
  try {
    const result = await releasesApi.getReleaseDiff(releaseId.value);
    if (seq !== diffLoadSeq) return;

    if (result.release.project_id !== projectId.value) {
      throw new ApiRequestError(404, {
        code: "release_not_found",
        message: t("releases.detail.notFound"),
      });
    }

    diff.value = result;
  } catch (err) {
    if (seq !== diffLoadSeq) return;
    if (err instanceof ApiRequestError) {
      diffError.value = err;
    } else {
      diffError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("releases.diff.loadError"),
      });
    }
  } finally {
    if (seq === diffLoadSeq) {
      diffLoading.value = false;
    }
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  const seq = ++diffLoadSeq;
  await fetchProject(pid);
  if (seq !== diffLoadSeq) return;
  await loadDiff();
}

function goBackToDetail() {
  router.push({
    name: ROUTE_NAMES.RELEASE_DETAIL,
    params: {
      projectId: route.params.projectId,
      releaseId: route.params.releaseId,
    },
  });
}

onMounted(loadAll);

watch(
  () => [route.params.projectId, route.params.releaseId],
  () => loadAll(),
);
</script>

<style scoped>
.release-diff-page {
  width: 100%;
}

.release-diff-page__section {
  margin-top: var(--spacing-md);
}

.release-diff-page__meta {
  margin-bottom: var(--spacing-md);
}

.release-diff-page__code {
  font-family: monospace;
}

.release-diff-page__alert {
  margin-bottom: var(--spacing-md);
}

.release-diff-page__content {
  margin-top: var(--spacing-md);
}
</style>
