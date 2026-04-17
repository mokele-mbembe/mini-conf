<template>
  <div class="project-section-placeholder-page page-container">
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
      <PageHeader :title="sectionTitle" :subtitle="sectionSubtitle">
        <template #actions>
          <StatusBadge :status="project.status" />
        </template>
      </PageHeader>

      <ProjectTabs />

      <el-card class="project-section-placeholder-page__card" shadow="never">
        <h2 class="project-section-placeholder-page__heading">
          {{ t("projectSection.placeholder.heading") }}
        </h2>
        <p class="project-section-placeholder-page__body">
          {{ sectionDescription }}
        </p>
      </el-card>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
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
const { t } = useI18nText();
const { project, loading, error, fetchProject } = useProjectContext();

const sectionKey = computed(() => {
  const name = route.name as string | undefined;

  switch (name) {
    case "DeploymentList":
      return "deployments";
    case "ReleaseList":
      return "releases";
    case "ProjectMembers":
      return "members";
    case "SyncRecordList":
      return "syncRecords";
    case "HeartbeatList":
      return "heartbeats";
    case "AuditLogList":
      return "auditLogs";
    default:
      return "overview";
  }
});

const sectionTitle = computed(() =>
  t(`projectSection.${sectionKey.value}.title`),
);
const sectionSubtitle = computed(() =>
  t(`projectSection.${sectionKey.value}.subtitle`, {
    project: project.value?.name ?? "",
  }),
);
const sectionDescription = computed(() =>
  t(`projectSection.${sectionKey.value}.description`),
);

function loadProject() {
  const id = Number(route.params.projectId);
  if (!Number.isNaN(id)) {
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
.project-section-placeholder-page {
  width: 100%;
}

.project-section-placeholder-page__card {
  border: 1px dashed var(--color-border-light);
  background: linear-gradient(
    180deg,
    rgba(64, 158, 255, 0.04),
    rgba(64, 158, 255, 0)
  );
}

.project-section-placeholder-page__heading {
  margin: 0 0 var(--spacing-sm);
  font-size: var(--font-size-lg);
  font-weight: 600;
}

.project-section-placeholder-page__body {
  margin: 0;
  color: var(--color-text-secondary);
  line-height: 1.6;
}
</style>
