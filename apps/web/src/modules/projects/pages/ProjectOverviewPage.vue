<template>
  <div class="project-overview-page">
    <LoadingState v-if="loading" />

    <NotFoundState
      v-else-if="error && error.status === 404"
      :title="t('project.notFound.title')"
      :subtitle="t('project.notFound.subtitle')"
    />

    <ForbiddenState
      v-else-if="error && error.status === 403"
      :subtitle="t('project.forbidden.subtitle')"
    />

    <ErrorState
      v-else-if="error"
      :title="t('project.loadFailed')"
      :subtitle="getErrorMessage(error.code, error.message)"
      @retry="loadProject"
    />

    <template v-else-if="project">
      <PageHeader
        :title="project.name"
        :subtitle="project.description ?? undefined"
      >
        <template #actions>
          <StatusBadge :status="project.status" />
        </template>
      </PageHeader>

      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('project.field.code')">
          {{ project.code }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('project.field.status')">
          <StatusBadge :status="project.status" />
        </el-descriptions-item>
        <el-descriptions-item :label="t('project.field.description')" :span="2">
          {{ project.description || t("project.emptyDescription") }}
        </el-descriptions-item>
      </el-descriptions>

      <ProjectTabs />
    </template>
  </div>
</template>

<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { useProjectContext } from "../composables/useProjectContext";
import ProjectTabs from "../components/ProjectTabs.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import { useI18nText } from "@/shared/i18n";
import { getErrorMessage } from "@/shared/constants/error-messages";

const route = useRoute();
const { project, loading, error, fetchProject } = useProjectContext();
const { t } = useI18nText();

function loadProject() {
  const id = Number(route.params.projectId);
  if (!isNaN(id)) {
    fetchProject(id);
  }
}

onMounted(loadProject);

watch(
  () => route.params.projectId,
  () => loadProject(),
);
</script>

<style scoped>
.project-overview-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}

@media (max-width: 768px) {
  .project-overview-page {
    padding: var(--spacing-md);
  }
}
</style>
