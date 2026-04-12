<template>
  <div class="project-overview-page">
    <LoadingState v-if="loading" />

    <NotFoundState
      v-else-if="error && error.status === 404"
      title="项目未找到"
      subtitle="请求的项目不存在"
    />

    <ForbiddenState
      v-else-if="error && error.status === 403"
      subtitle="你当前角色没有查看该项目的权限"
    />

    <ErrorState v-else-if="error" :title="error.message" @retry="loadProject" />

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
        <el-descriptions-item label="编码">
          {{ project.code }}
        </el-descriptions-item>
        <el-descriptions-item label="状态">
          <StatusBadge :status="project.status" />
        </el-descriptions-item>
        <el-descriptions-item label="描述" :span="2">
          {{ project.description || "暂无描述" }}
        </el-descriptions-item>
      </el-descriptions>

      <ProjectTabs @navigate="handleTabNavigate" />

      <div class="project-overview-page__placeholder">
        <EmptyState description="子页面将在后续版本中实现" />
      </div>
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
import EmptyState from "@/shared/states/EmptyState.vue";

const route = useRoute();
const { project, loading, error, fetchProject } = useProjectContext();

function loadProject() {
  const id = Number(route.params.projectId);
  if (!isNaN(id)) {
    fetchProject(id);
  }
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
function handleTabNavigate(tabName: string) {
  // Placeholder: future versions will navigate to sub-routes
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
.project-overview-page__placeholder {
  margin-top: var(--spacing-lg);
}

@media (max-width: 768px) {
  .project-overview-page {
    padding: var(--spacing-md);
  }
}
</style>
