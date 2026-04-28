<template>
  <div
    class="draft-editor-page"
    :class="embedded ? 'draft-editor-page--embedded' : 'page-container'"
  >
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
      <PageHeader :title="pageTitle" :subtitle="pageSubtitle">
        <template #actions>
          <div class="draft-editor-page__header-actions">
            <el-button v-if="embedded" text :icon="Close" @click="requestClose">
              {{ t("common.close") }}
            </el-button>
            <el-button v-if="canEdit" text type="primary" @click="goToPreview">
              {{ t("drafts.action.preview") }}
            </el-button>
            <el-button
              v-if="canPublish"
              type="success"
              :loading="publishing"
              :disabled="resourceLoading || !draftReady"
              @click="handlePublish"
            >
              {{ t("drafts.action.publish") }}
            </el-button>
            <el-button
              v-if="canEdit"
              type="primary"
              :loading="saving"
              :disabled="resourceLoading"
              @click="handleSave"
            >
              {{ t("common.save") }}
            </el-button>
          </div>
        </template>
      </PageHeader>

      <ProjectTabs v-if="!embedded" />

      <div class="draft-editor-page__section">
        <div v-if="!embedded" class="draft-editor-page__nav">
          <el-button text type="primary" @click="backToDeployment">
            {{ t("drafts.action.backToDeployment") }}
          </el-button>
        </div>

        <LoadingState v-if="resourceLoading" />

        <NotFoundState
          v-else-if="resourceNotFound"
          :title="t('drafts.notFound.title')"
          :subtitle="t('drafts.notFound.subtitle')"
        />

        <ForbiddenState
          v-else-if="!canEdit || resourceForbidden"
          :subtitle="t('state.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="resourceError"
          :title="t('drafts.page.loadError')"
          :subtitle="getErrorMessage(resourceError.code, resourceError.message)"
          @retry="loadDraftResources"
        />

        <template v-else-if="deployment && configFile">
          <ConfigFileSwitcher
            :config-files="configFiles"
            :current-config-file-id="configFileId"
            :preview-status-map="previewStatusMap"
            @switch="switchConfigFile"
          />

          <DraftWorkspaceSummary
            :deployment="deployment"
            :config-file="configFile"
            :version-label="versionLabel"
            :can-edit="canEdit"
            :draft-ready="draftReady"
            :discarding="discarding"
            :restoring="restoring"
            @discard="handleDiscard"
            @restore-from-release="handleRestoreFromRelease"
            @clone-from-instance="cloneDialogVisible = true"
          />

          <el-alert
            v-if="deployment.is_template"
            :title="t('drafts.notice.template')"
            type="info"
            show-icon
            :closable="false"
            class="draft-editor-page__notice"
          />

          <el-alert
            v-if="isDirty"
            :title="t('drafts.notice.unsaved')"
            type="warning"
            show-icon
            :closable="false"
            class="draft-editor-page__notice"
          />

          <ConfigWorkspaceLayout>
            <template #main>
              <ConfigCodeEditor
                v-model="content"
                :format="configFile.format"
                :placeholder="t('drafts.editor.placeholder')"
                :aria-label="t('drafts.editor.ariaLabel')"
                class="draft-editor-page__editor"
              />
            </template>

            <template v-if="canViewSavedVersions" #aside>
              <DraftSavedVersionsPanel
                v-model:note="savedVersionNote"
                class="draft-editor-page__history"
                :saved-versions="savedVersions"
                :loading="savedVersionsLoading"
                :error="savedVersionsError"
                :selected-saved-version-id="selectedSavedVersionId"
                :saved-version-detail="savedVersionDetail"
                :detail-loading="savedVersionDetailLoading"
                :note-max-length="SAVED_VERSION_NOTE_MAX_LENGTH"
                :updating-note="updatingSavedVersionNote"
                :restoring="restoringFromSavedVersion"
                :deleting="deletingSavedVersion"
                @select="selectSavedVersion"
                @save-note="handleUpdateSavedVersionNote"
                @restore="handleRestoreSavedVersion"
                @delete="handleDeleteSavedVersion"
              />
            </template>
          </ConfigWorkspaceLayout>
        </template>
      </div>
    </template>

    <DraftCloneSourceDialog
      v-model="cloneDialogVisible"
      v-model:source-instance-id="cloneSourceInstanceId"
      v-model:source-kind="cloneSourceKind"
      :clone-sources="cloneSources"
      :load-error="cloneLoadError"
      :instances-loading="cloneInstancesLoading"
      :loading-more="cloneLoadingMore"
      :next-cursor="cloneNextCursor"
      :selected-source="selectedCloneSource"
      :draft-option-disabled="cloneDraftOptionDisabled"
      :release-option-disabled="cloneReleaseOptionDisabled"
      :selected-source-draft-unavailable="selectedCloneSourceDraftUnavailable"
      :selected-source-release-unavailable="
        selectedCloneSourceReleaseUnavailable
      "
      :submit-disabled="cloneSubmitDisabled"
      :cloning="cloning"
      @search="handleCloneRemoteSearch"
      @load-more="loadMoreCloneSources"
      @submit="handleCloneFromInstance"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { Close } from "@element-plus/icons-vue";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ConfigFileSwitcher from "@/modules/config-workspace/components/ConfigFileSwitcher.vue";
import ConfigCodeEditor from "@/modules/config-workspace/components/ConfigCodeEditor.vue";
import ConfigWorkspaceLayout from "@/modules/config-workspace/components/ConfigWorkspaceLayout.vue";
import DraftCloneSourceDialog from "@/modules/drafts/components/DraftCloneSourceDialog.vue";
import DraftSavedVersionsPanel from "@/modules/drafts/components/DraftSavedVersionsPanel.vue";
import DraftWorkspaceSummary from "@/modules/drafts/components/DraftWorkspaceSummary.vue";
import {
  SAVED_VERSION_NOTE_MAX_LENGTH,
  useSavedVersionsPanel,
} from "@/modules/drafts/composables/useSavedVersionsPanel";
import { useDraftCloneDialog } from "@/modules/drafts/composables/useDraftCloneDialog";
import { useDraftActions } from "@/modules/drafts/composables/useDraftActions";
import { useDraftWorkspaceResources } from "@/modules/drafts/composables/useDraftWorkspaceResources";
import { useDraftUnsavedChangesGuard } from "@/modules/drafts/composables/useDraftUnsavedChangesGuard";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const router = useRouter();
const { t } = useI18nText();

const props = withDefaults(
  defineProps<{
    embedded?: boolean;
    configFileIdOverride?: number | null;
  }>(),
  {
    embedded: false,
    configFileIdOverride: null,
  },
);

const emit = defineEmits<{
  close: [];
  "switch-config": [configFileId: number];
}>();

const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));
const deploymentId = computed(() => Number(route.params.deploymentId));
const configFileId = computed(
  () => props.configFileIdOverride ?? Number(route.params.configFileId),
);
const canEdit = computed(() => {
  const role = project.value?.current_user_role;
  return role === "admin" || role === "editor";
});

const canViewSavedVersions = computed(() => canEdit.value);

const {
  deployment,
  configFile,
  configFiles,
  draft,
  content,
  resourceLoading,
  resourceError,
  previewStatusMap,
  resourceNotFound,
  resourceForbidden,
  draftReady,
  isDirty,
  draftVersion,
  versionLabel,
  loadDraftResources: loadDraftWorkspaceResources,
  loadPreviewStatus,
  applyDraft,
  markDraftMissing,
} = useDraftWorkspaceResources({
  projectId,
  deploymentId,
  configFileId,
  canEdit,
  canViewSavedVersions,
  t,
});

const canPublish = computed(
  () =>
    canEdit.value && deployment.value !== null && !deployment.value.is_template,
);
const pageTitle = computed(() => {
  if (configFile.value) {
    return t("drafts.page.title", { config: configFile.value.code });
  }

  return t("drafts.page.fallbackTitle");
});
const pageSubtitle = computed(() => {
  if (!deployment.value) {
    return undefined;
  }

  return `${deployment.value.name} / ${deployment.value.deployment_key}`;
});

const {
  savedVersions,
  savedVersionsLoading,
  savedVersionsError,
  selectedSavedVersionId,
  savedVersionDetail,
  savedVersionDetailLoading,
  savedVersionNote,
  updatingSavedVersionNote,
  restoringFromSavedVersion,
  deletingSavedVersion,
  resetSavedVersions,
  loadSavedVersions,
  selectSavedVersion,
  handleUpdateSavedVersionNote,
  handleRestoreSavedVersion,
  handleDeleteSavedVersion,
} = useSavedVersionsPanel({
  canViewSavedVersions,
  deploymentId,
  configFileId,
  draftVersion,
  isDirty,
  applyDraft,
  refreshPreviewStatus: loadPreviewStatus,
  t,
});

const {
  cloneDialogVisible,
  cloneSourceInstanceId,
  cloneSourceKind,
  cloneSources,
  cloneInstancesLoading,
  cloneLoadingMore,
  cloneLoadError,
  cloning,
  cloneNextCursor,
  selectedCloneSource,
  cloneDraftOptionDisabled,
  cloneReleaseOptionDisabled,
  selectedCloneSourceDraftUnavailable,
  selectedCloneSourceReleaseUnavailable,
  cloneSubmitDisabled,
  loadMoreCloneSources,
  handleCloneRemoteSearch,
  handleCloneFromInstance,
} = useDraftCloneDialog({
  projectId,
  deploymentId,
  configFileId,
  isDirty,
  applyDraft,
  refreshPreviewStatus: loadPreviewStatus,
  t,
});

const {
  saving,
  publishing,
  discarding,
  restoring,
  handleSave,
  handleDiscard,
  handleRestoreFromRelease,
  handlePublish,
} = useDraftActions({
  projectId,
  deploymentId,
  configFileId,
  configFile,
  draft,
  content,
  isDirty,
  applyDraft,
  markDraftMissing,
  loadSavedVersions,
  refreshPreviewStatus: loadPreviewStatus,
  onReleasePublished: (release) => {
    void router.push({
      name: ROUTE_NAMES.RELEASE_DETAIL,
      params: { projectId: route.params.projectId, releaseId: release.id },
    });
  },
  t,
});

const { confirmIfDirty } = useDraftUnsavedChangesGuard({
  isDirty,
  t,
});

async function switchConfigFile(cfId: number) {
  if (!(await confirmIfDirty())) return;
  if (props.embedded) {
    emit("switch-config", cfId);
    return;
  }

  router.push({
    name: ROUTE_NAMES.DRAFT_EDITOR,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
      configFileId: String(cfId),
    },
  });
}

async function requestClose() {
  if (await confirmIfDirty()) {
    emit("close");
  }
}

defineExpose({
  requestClose,
});

async function loadDraftResources() {
  await loadDraftWorkspaceResources({
    resetSavedVersions,
    loadSavedVersions,
  });
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  await fetchProject(pid);
  await loadDraftResources();
}

function backToDeployment() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_DETAIL,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
  });
}

function goToPreview() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
  });
}

onMounted(loadAll);

watch(
  () => [projectId.value, deploymentId.value, configFileId.value],
  () => loadAll(),
);
</script>

<style scoped>
.draft-editor-page {
  width: 100%;
}

.draft-editor-page--embedded {
  min-height: 100%;
  padding: var(--spacing-lg);
  background: var(--color-bg-card);
}

.draft-editor-page__section {
  margin-top: var(--spacing-md);
}

.draft-editor-page__header-actions,
.draft-editor-page__nav {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  justify-content: flex-end;
}

.draft-editor-page__nav {
  justify-content: flex-start;
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__notice {
  margin-bottom: var(--spacing-md);
}
</style>
