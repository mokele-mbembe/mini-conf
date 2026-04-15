<template>
  <div class="deployment-instance-detail-page">
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
      <PageHeader :title="detailTitle" :subtitle="detailSubtitle">
        <template #actions>
          <StatusBadge v-if="deployment" :status="deployment.status" />
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="deployment-instance-detail-page__section">
        <el-button text type="primary" @click="backToList">
          {{ t("deployments.detail.back") }}
        </el-button>

        <LoadingState v-if="detailLoading" />

        <NotFoundState
          v-else-if="isProjectMismatch || detailError?.status === 404"
          :title="t('deployments.notFound.title')"
          :subtitle="t('deployments.notFound.subtitle')"
        />

        <ForbiddenState
          v-else-if="detailError?.status === 403"
          :subtitle="t('project.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="detailError"
          :title="t('deployments.detail.loadError')"
          :subtitle="getErrorMessage(detailError.code, detailError.message)"
          @retry="loadDeploymentInstance"
        />

        <template v-else-if="deployment">
          <div class="deployment-instance-detail-page__summary">
            <div>
              <div class="deployment-instance-detail-page__eyebrow">
                {{ t("deployments.column.deploymentKey") }}
              </div>
              <div class="deployment-instance-detail-page__key">
                {{ deployment.deployment_key }}
              </div>
            </div>

            <div class="deployment-instance-detail-page__badges">
              <el-tag v-if="deployment.is_template" size="small" type="warning">
                {{ t("deployments.type.template") }}
              </el-tag>
              <el-tag v-else size="small" type="success">
                {{ t("deployments.type.instance") }}
              </el-tag>
              <StatusBadge :status="deployment.status" />
            </div>
          </div>

          <el-descriptions :column="2" border>
            <el-descriptions-item :label="t('deployments.field.id')">
              {{ deployment.id }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.environment')">
              <el-tag size="small" type="info">
                {{ deployment.environment }}
              </el-tag>
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.deploymentKey')">
              <span class="deployment-instance-detail-page__code">
                {{ deployment.deployment_key }}
              </span>
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.name')">
              {{ deployment.name }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.type')">
              {{ deploymentTypeLabel }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.status')">
              <StatusBadge :status="deployment.status" />
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('deployments.field.templateSource')"
            >
              {{ templateSourceLabel }}
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('deployments.field.description')"
              :span="2"
            >
              {{ deployment.description || t("deployments.emptyDescription") }}
            </el-descriptions-item>
          </el-descriptions>
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
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
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
const deploymentId = computed(() => Number(route.params.deploymentId));

const deployment = ref<DeploymentInstanceSummary | null>(null);
const detailLoading = ref(false);
const detailError = ref<ApiRequestError | null>(null);
const detailTitle = computed(
  () => deployment.value?.name ?? t("deployments.detail.title"),
);
const detailSubtitle = computed(
  () => deployment.value?.description ?? undefined,
);
const deploymentTypeLabel = computed(() => {
  if (!deployment.value) {
    return "";
  }

  return deployment.value.is_template
    ? t("deployments.type.template")
    : t("deployments.type.instance");
});
const templateSourceLabel = computed(() => {
  if (!deployment.value) {
    return "";
  }

  return deployment.value.template_source_id == null
    ? t("deployments.emptyTemplateSource")
    : String(deployment.value.template_source_id);
});
const isProjectMismatch = computed(
  () =>
    deployment.value !== null &&
    !Number.isNaN(projectId.value) &&
    deployment.value.project_id !== projectId.value,
);

async function loadDeploymentInstance() {
  const id = deploymentId.value;
  if (Number.isNaN(id)) return;

  detailLoading.value = true;
  detailError.value = null;
  deployment.value = null;
  try {
    deployment.value = await deploymentInstancesApi.getDeploymentInstance(id);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      detailError.value = err;
    } else {
      detailError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("deployments.detail.loadError"),
      });
    }
  } finally {
    detailLoading.value = false;
  }
}

async function loadAll() {
  const id = projectId.value;
  if (Number.isNaN(id)) return;
  await fetchProject(id);
  await loadDeploymentInstance();
}

function backToList() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_LIST,
    params: { projectId: route.params.projectId },
  });
}

onMounted(loadAll);

watch(
  () => [route.params.projectId, route.params.deploymentId],
  () => loadAll(),
);
</script>

<style scoped>
.deployment-instance-detail-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}

.deployment-instance-detail-page__section {
  margin-top: var(--spacing-md);
}

.deployment-instance-detail-page__summary {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin: var(--spacing-md) 0;
  padding-bottom: var(--spacing-md);
  border-bottom: 1px solid var(--color-border-light);
}

.deployment-instance-detail-page__eyebrow {
  margin-bottom: 4px;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.deployment-instance-detail-page__key,
.deployment-instance-detail-page__code {
  font-family: monospace;
}

.deployment-instance-detail-page__key {
  font-size: var(--font-size-lg);
  font-weight: 600;
}

.deployment-instance-detail-page__badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  justify-content: flex-end;
}

@media (max-width: 768px) {
  .deployment-instance-detail-page {
    padding: var(--spacing-md);
  }

  .deployment-instance-detail-page__summary {
    flex-direction: column;
  }
}
</style>
