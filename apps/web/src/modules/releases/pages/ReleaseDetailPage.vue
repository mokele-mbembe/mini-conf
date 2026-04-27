<template>
  <div class="release-detail-page page-container">
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
        :title="t('releases.detail.title')"
        :subtitle="t('releases.detail.subtitle', { project: project.name })"
      >
        <template #actions>
          <el-button @click="goBack">
            {{ t("releases.detail.backToList") }}
          </el-button>
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="release-detail-page__section">
        <LoadingState v-if="detailLoading" />

        <NotFoundState
          v-else-if="detailError && detailError.status === 404"
          :title="t('releases.detail.notFound')"
          :subtitle="t('releases.detail.notFoundHint')"
        />

        <ForbiddenState
          v-else-if="detailError && detailError.status === 403"
          :subtitle="t('project.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="detailError"
          :title="t('releases.detail.loadError')"
          :subtitle="getErrorMessage(detailError.code, detailError.message)"
          @retry="loadDetail"
        />

        <template v-else-if="detail">
          <!-- Meta info -->
          <el-descriptions :column="2" border class="release-detail-page__meta">
            <el-descriptions-item :label="t('releases.column.revision')">
              <span class="release-detail-page__code">
                {{ detail.release.revision }}
              </span>
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.detail.contentHash')">
              <span class="release-detail-page__code">
                {{ detail.release.content_hash }}
              </span>
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('deployments.column.deploymentKey')"
            >
              {{ deploymentLabel }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('configFiles.column.code')">
              {{ configLabel }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('configFiles.column.format')">
              <el-tag size="small" type="info">
                {{ detail.release.format }}
              </el-tag>
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.detail.applyMode')">
              {{ detail.release.apply_mode }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.detail.publishedBy')">
              {{ detail.release.published_by }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('releases.column.publishedAt')">
              {{ detail.release.published_at }}
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('releases.column.changeSummary')"
              :span="2"
            >
              {{ changeSummaryText }}
            </el-descriptions-item>

            <el-descriptions-item
              v-if="detail.diff_summary"
              :label="t('releases.detail.diffSummary')"
              :span="2"
            >
              <template v-if="detail.diff_summary.is_initial">
                {{ t("releases.diff.firstRelease") }}
              </template>
              <template v-else>
                +{{ detail.diff_summary.added_lines }} / -{{
                  detail.diff_summary.removed_lines
                }}
              </template>
            </el-descriptions-item>
          </el-descriptions>

          <!-- Redacted hint -->
          <el-alert
            v-if="detail.content_redacted"
            type="warning"
            :closable="false"
            show-icon
            class="release-detail-page__alert"
          >
            {{ t("releases.detail.redactedHint") }}
          </el-alert>

          <!-- Readonly hint -->
          <el-alert
            type="info"
            :closable="false"
            show-icon
            class="release-detail-page__alert"
          >
            {{ t("releases.detail.readonlyHint") }}
          </el-alert>

          <!-- Content -->
          <ConfigCodeEditor
            :model-value="detail.content"
            :format="detail.release.format"
            :readonly="true"
            :min-height="420"
            :aria-label="t('releases.detail.contentAriaLabel')"
            class="release-detail-page__content"
          />

          <!-- Actions -->
          <div class="release-detail-page__actions">
            <el-button type="primary" @click="goToDiff">
              {{ t("releases.detail.viewDiff") }}
            </el-button>
            <el-button @click="copyContent">
              {{ t("releases.detail.copyContent") }}
            </el-button>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ConfigCodeEditor from "@/modules/config-workspace/components/ConfigCodeEditor.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as releasesApi from "@/api/releases";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import type { ReleaseDetailResponse } from "@/api/types/release";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { ConfigFileSummary } from "@/api/types/config-file";
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

const detail = ref<ReleaseDetailResponse | null>(null);
const deployment = ref<DeploymentInstanceSummary | null>(null);
const configFile = ref<ConfigFileSummary | null>(null);
const detailLoading = ref(false);
const detailError = ref<ApiRequestError | null>(null);
let detailLoadSeq = 0;

const deploymentLabel = computed(() => {
  if (!detail.value) return "";
  const d = deployment.value;
  return d
    ? `${d.name} / ${d.deployment_key}`
    : String(detail.value.release.deployment_instance_id);
});

const configLabel = computed(() => {
  if (!detail.value) return "";
  const c = configFile.value;
  return c
    ? `${c.name} / ${c.code}`
    : String(detail.value.release.config_file_id);
});

const changeSummaryText = computed(() => {
  return (
    detail.value?.release.change_summary || t("releases.emptyChangeSummary")
  );
});

async function loadDetail() {
  const seq = ++detailLoadSeq;
  detailLoading.value = true;
  detailError.value = null;
  try {
    const detailResult = await releasesApi.getReleaseDetail(releaseId.value);
    if (seq !== detailLoadSeq) return;

    if (detailResult.release.project_id !== projectId.value) {
      throw new ApiRequestError(404, {
        code: "release_not_found",
        message: t("releases.detail.notFound"),
      });
    }

    const [deploymentResult, configResult] = await Promise.all([
      deploymentInstancesApi.getDeploymentInstance(
        detailResult.release.deployment_instance_id,
      ),
      configFilesApi.getConfigFile(detailResult.release.config_file_id),
    ]);
    if (seq !== detailLoadSeq) return;
    detail.value = detailResult;
    deployment.value = deploymentResult;
    configFile.value = configResult;
  } catch (err) {
    if (seq !== detailLoadSeq) return;
    if (err instanceof ApiRequestError) {
      detailError.value = err;
    } else {
      detailError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("releases.detail.loadError"),
      });
    }
  } finally {
    if (seq === detailLoadSeq) {
      detailLoading.value = false;
    }
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  const seq = ++detailLoadSeq;
  await fetchProject(pid);
  if (seq !== detailLoadSeq) return;
  await loadDetail();
}

function goBack() {
  router.push({
    name: ROUTE_NAMES.RELEASE_LIST,
    params: { projectId: route.params.projectId },
  });
}

function goToDiff() {
  router.push({
    name: ROUTE_NAMES.RELEASE_DIFF,
    params: {
      projectId: route.params.projectId,
      releaseId: route.params.releaseId,
    },
  });
}

async function copyContent() {
  if (!detail.value) return;
  try {
    await globalThis.navigator.clipboard.writeText(detail.value.content);
    ElMessage.success(t("releases.detail.contentCopied"));
  } catch {
    ElMessage.error(t("toast.operationFailed"));
  }
}

onMounted(loadAll);

watch(
  () => [route.params.projectId, route.params.releaseId],
  () => loadAll(),
);
</script>

<style scoped>
.release-detail-page {
  width: 100%;
}

.release-detail-page__section {
  margin-top: var(--spacing-md);
}

.release-detail-page__meta {
  margin-bottom: var(--spacing-md);
}

.release-detail-page__code {
  font-family: monospace;
}

.release-detail-page__alert {
  margin-bottom: var(--spacing-md);
}

.release-detail-page__content {
  margin-bottom: var(--spacing-md);
}

.release-detail-page__actions {
  display: flex;
  gap: var(--spacing-sm);
}
</style>
