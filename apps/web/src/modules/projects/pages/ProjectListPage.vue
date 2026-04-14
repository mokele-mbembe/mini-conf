<template>
  <div class="project-list-page">
    <PageHeader
      :title="t('projects.list.title')"
      :subtitle="t('projects.list.subtitle')"
    />

    <LoadingState v-if="loading" />

    <ErrorState
      v-else-if="error"
      :title="t('projects.list.loadFailed')"
      :subtitle="getErrorMessage(error.code, error.message)"
      @retry="fetchProjects"
    />

    <EmptyState
      v-else-if="projects.length === 0"
      :description="t('projects.list.empty')"
    />

    <div v-else class="project-list-page__grid">
      <el-card
        v-for="item in projects"
        :key="item.id"
        shadow="hover"
        class="project-list-page__card"
        @click="goToProject(item.id)"
      >
        <template #header>
          <div class="project-list-page__card-header">
            <span class="project-list-page__card-code">{{ item.code }}</span>
            <StatusBadge :status="item.status" />
          </div>
        </template>
        <h3 class="project-list-page__card-name">{{ item.name }}</h3>
        <p class="project-list-page__card-desc">
          {{ item.description || t("project.emptyDescription") }}
        </p>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import * as projectsApi from "@/api/projects";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ProjectSummary } from "@/api/types/project";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";

const router = useRouter();
const { t } = useI18nText();

const projects = ref<ProjectSummary[]>([]);
const loading = ref(false);
const error = ref<ApiRequestError | null>(null);

async function fetchProjects() {
  loading.value = true;
  error.value = null;
  try {
    const res = await projectsApi.listProjects();
    projects.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      error.value = err;
    } else {
      error.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("projects.list.loadFailed"),
      });
    }
  } finally {
    loading.value = false;
  }
}

function goToProject(id: number) {
  router.push({
    name: ROUTE_NAMES.PROJECT_OVERVIEW,
    params: { projectId: String(id) },
  });
}

onMounted(fetchProjects);
</script>

<style scoped>
.project-list-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}
.project-list-page__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--spacing-md);
}
.project-list-page__card {
  cursor: pointer;
  transition: transform 0.15s;
}
.project-list-page__card:hover {
  transform: translateY(-2px);
}
.project-list-page__card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.project-list-page__card-code {
  font-weight: 600;
  font-size: var(--font-size-base);
  font-family: monospace;
}
.project-list-page__card-name {
  font-size: var(--font-size-lg);
  font-weight: 500;
  margin-bottom: var(--spacing-sm);
}
.project-list-page__card-desc {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  line-height: 1.5;
}

@media (max-width: 768px) {
  .project-list-page {
    padding: var(--spacing-md);
  }
  .project-list-page__grid {
    grid-template-columns: 1fr;
  }
}
</style>
