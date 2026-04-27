<template>
  <div class="draft-editor-page page-container">
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

      <ProjectTabs />

      <div class="draft-editor-page__section">
        <div class="draft-editor-page__nav">
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

          <div class="draft-editor-page__meta">
            <el-descriptions :column="3" border>
              <el-descriptions-item :label="t('deployments.field.name')">
                {{ deployment.name }}
              </el-descriptions-item>
              <el-descriptions-item
                :label="t('deployments.field.deploymentKey')"
              >
                <span class="draft-editor-page__code">
                  {{ deployment.deployment_key }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item :label="t('deployments.field.type')">
                {{
                  deployment.is_template
                    ? t("deployments.type.template")
                    : t("deployments.type.instance")
                }}
              </el-descriptions-item>
              <el-descriptions-item :label="t('configFiles.column.code')">
                <span class="draft-editor-page__code">
                  {{ configFile.code }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item :label="t('configFiles.column.format')">
                <el-tag size="small" type="info">
                  {{ configFile.format }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item :label="t('drafts.field.version')">
                {{ versionLabel }}
              </el-descriptions-item>
            </el-descriptions>
          </div>

          <!-- Draft actions toolbar -->
          <div v-if="canEdit" class="draft-editor-page__draft-actions">
            <el-button
              v-if="draftReady"
              text
              type="danger"
              size="small"
              :loading="discarding"
              @click="handleDiscard"
            >
              {{ t("drafts.action.discard") }}
            </el-button>
            <el-button
              text
              type="primary"
              size="small"
              :loading="restoring"
              @click="handleRestoreFromRelease"
            >
              {{ t("drafts.action.restoreFromRelease") }}
            </el-button>
            <el-button
              text
              type="primary"
              size="small"
              @click="cloneDialogVisible = true"
            >
              {{ t("drafts.action.cloneFromInstance") }}
            </el-button>
          </div>

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

    <!-- Clone from other instance dialog -->
    <el-dialog
      v-model="cloneDialogVisible"
      :title="t('drafts.cloneDialog.title')"
      width="520px"
      destroy-on-close
    >
      <el-alert
        v-if="cloneLoadError"
        type="error"
        :closable="false"
        style="margin-bottom: 16px"
      >
        {{ t("drafts.cloneDialog.loadError") }}
      </el-alert>
      <el-form label-position="top">
        <el-form-item :label="t('drafts.cloneDialog.sourceInstance')">
          <el-select
            v-model="cloneSourceInstanceId"
            :placeholder="t('drafts.cloneDialog.selectInstance')"
            filterable
            remote
            :remote-method="handleCloneRemoteSearch"
            style="width: 100%"
            :loading="cloneInstancesLoading"
          >
            <el-option
              v-for="src in cloneSources"
              :key="src.deployment_instance_id"
              :label="
                src.is_template
                  ? `${src.name} (${src.deployment_key}) [${t('drafts.cloneDialog.templateTag')}]`
                  : `${src.name} (${src.deployment_key})`
              "
              :value="src.deployment_instance_id"
            >
              <div
                style="
                  display: flex;
                  justify-content: space-between;
                  align-items: center;
                "
              >
                <span>
                  {{ src.name }}
                  <span style="color: var(--el-text-color-secondary)">
                    ({{ src.deployment_key }})
                  </span>
                  <el-tag
                    v-if="src.is_template"
                    size="small"
                    type="info"
                    style="margin-left: 4px"
                  >
                    {{ t("drafts.cloneDialog.templateTag") }}
                  </el-tag>
                </span>
                <span
                  style="font-size: 12px; color: var(--el-text-color-secondary)"
                >
                  <template v-if="cloneSourceHasNoAvailableSources(src)">
                    {{ t("drafts.cloneDialog.noSources") }}
                  </template>
                  <template v-else>
                    <span
                      v-if="src.available_sources.draft"
                      style="margin-right: 6px"
                    >
                      Draft ✓
                    </span>
                    <span v-if="src.available_sources.latest_release">
                      Release ✓
                    </span>
                  </template>
                </span>
              </div>
            </el-option>
            <template v-if="cloneNextCursor" #footer>
              <el-button
                text
                :loading="cloneLoadingMore"
                style="width: 100%"
                @mousedown.prevent
                @click="loadMoreCloneSources"
              >
                {{ t("drafts.cloneDialog.loadMore") }}
              </el-button>
            </template>
          </el-select>
        </el-form-item>
        <el-form-item :label="t('drafts.cloneDialog.sourceKind')">
          <el-radio-group v-model="cloneSourceKind">
            <el-radio value="draft" :disabled="cloneDraftOptionDisabled">
              {{ t("drafts.cloneDialog.kindDraft") }}
              <span
                v-if="selectedCloneSourceDraftUnavailable"
                style="color: var(--el-text-color-secondary); font-size: 12px"
              >
                ({{ t("drafts.cloneDialog.sourceUnavailable") }})
              </span>
            </el-radio>
            <el-radio
              value="latest_release"
              :disabled="cloneReleaseOptionDisabled"
            >
              {{ t("drafts.cloneDialog.kindRelease") }}
              <span
                v-if="selectedCloneSource?.is_template"
                style="color: var(--el-text-color-secondary); font-size: 12px"
              >
                ({{ t("drafts.cloneDialog.templateNoRelease") }})
              </span>
              <span
                v-else-if="selectedCloneSourceReleaseUnavailable"
                style="color: var(--el-text-color-secondary); font-size: 12px"
              >
                ({{ t("drafts.cloneDialog.sourceUnavailable") }})
              </span>
            </el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="cloneDialogVisible = false">
          {{ t("common.cancel") }}
        </el-button>
        <el-button
          type="primary"
          :loading="cloning"
          :disabled="cloneSubmitDisabled"
          @click="handleCloneFromInstance"
        >
          {{ t("drafts.cloneDialog.submit") }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ConfigFileSwitcher from "@/modules/config-workspace/components/ConfigFileSwitcher.vue";
import ConfigCodeEditor from "@/modules/config-workspace/components/ConfigCodeEditor.vue";
import ConfigWorkspaceLayout from "@/modules/config-workspace/components/ConfigWorkspaceLayout.vue";
import DraftSavedVersionsPanel from "@/modules/drafts/components/DraftSavedVersionsPanel.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import * as draftsApi from "@/api/drafts";
import * as releasesApi from "@/api/releases";
import * as savedVersionsApi from "@/api/saved-versions";
import * as cloneSourcesApi from "@/api/clone-sources";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { DraftResponse } from "@/api/types/draft";
import type { CloneSourceSummary } from "@/api/types/clone-source";
import type {
  SavedVersionDetail,
  SavedVersionSummary,
} from "@/api/types/saved-version";
import { useI18nText } from "@/shared/i18n";

const SAVED_VERSION_NOTE_MAX_LENGTH = 500;

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
const configFileId = computed(() => Number(route.params.configFileId));
const canEdit = computed(() => {
  const role = project.value?.current_user_role;
  return role === "admin" || role === "editor";
});

const deployment = ref<DeploymentInstanceSummary | null>(null);
const configFile = ref<ConfigFileSummary | null>(null);
const configFiles = ref<ConfigFileSummary[]>([]);
const draft = ref<DraftResponse | null>(null);
const content = ref("");
const savedContent = ref("");
const resourceLoading = ref(false);
const resourceError = ref<ApiRequestError | null>(null);
const saving = ref(false);
const publishing = ref(false);
const discarding = ref(false);
const restoring = ref(false);
const draftWasMissing = ref(false);

// Preview status per config (for switcher badges)
const previewStatusMap = ref<Record<number, string>>({});

// Clone from instance dialog
const cloneDialogVisible = ref(false);
const cloneSourceInstanceId = ref<number | null>(null);
const cloneSourceKind = ref<"draft" | "latest_release">("draft");
const cloneSources = ref<CloneSourceSummary[]>([]);
const cloneInstancesLoading = ref(false);
const cloneLoadingMore = ref(false);
const cloneLoadError = ref(false);
const cloning = ref(false);
const cloneNextCursor = ref<number | null>(null);
const cloneSearchKeyword = ref<string | undefined>(undefined);
let cloneSearchTimer: ReturnType<typeof globalThis.setTimeout> | null = null;
let cloneSearchSeq = 0;

const selectedCloneSource = computed(() =>
  cloneSources.value.find(
    (s) => s.deployment_instance_id === cloneSourceInstanceId.value,
  ),
);
const cloneDraftOptionDisabled = computed(
  () =>
    selectedCloneSource.value !== undefined &&
    !selectedCloneSource.value.available_sources.draft,
);
const cloneReleaseOptionDisabled = computed(
  () =>
    selectedCloneSource.value !== undefined &&
    (!selectedCloneSource.value.available_sources.latest_release ||
      selectedCloneSource.value.is_template),
);
const selectedCloneSourceDraftUnavailable = computed(
  () =>
    selectedCloneSource.value !== undefined &&
    !selectedCloneSource.value.available_sources.draft,
);
const selectedCloneSourceReleaseUnavailable = computed(
  () =>
    selectedCloneSource.value !== undefined &&
    !selectedCloneSource.value.available_sources.latest_release,
);

const cloneSubmitDisabled = computed(() => {
  if (!cloneSourceInstanceId.value || !selectedCloneSource.value) return true;
  const src = selectedCloneSource.value;
  if (cloneSourceKind.value === "draft" && !src.available_sources.draft)
    return true;
  if (
    cloneSourceKind.value === "latest_release" &&
    !src.available_sources.latest_release
  )
    return true;
  return false;
});

const savedVersions = ref<SavedVersionSummary[]>([]);
const savedVersionsLoading = ref(false);
const savedVersionsError = ref<ApiRequestError | null>(null);
const selectedSavedVersionId = ref<number | null>(null);
const savedVersionDetail = ref<SavedVersionDetail | null>(null);
const savedVersionDetailLoading = ref(false);
const savedVersionNote = ref("");
const savedNoteSnapshot = ref("");
const updatingSavedVersionNote = ref(false);
const restoringFromSavedVersion = ref(false);
const deletingSavedVersion = ref(false);

const resourceNotFound = computed(() => resourceError.value?.status === 404);
const resourceForbidden = computed(() => resourceError.value?.status === 403);
const draftReady = computed(() => draft.value !== null);
const isDirty = computed(() => content.value !== savedContent.value);
const canPublish = computed(
  () =>
    canEdit.value && deployment.value !== null && !deployment.value.is_template,
);
const canViewSavedVersions = computed(() => canEdit.value);
const isNoteDirty = computed(
  () =>
    savedVersionDetail.value !== null &&
    savedVersionNote.value !== savedNoteSnapshot.value,
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
const versionLabel = computed(() => {
  if (draft.value) {
    return String(draft.value.version);
  }

  return draftWasMissing.value
    ? t("drafts.field.newDraft")
    : t("drafts.field.unknownVersion");
});

function cloneSourceHasNoAvailableSources(src: CloneSourceSummary): boolean {
  return !src.available_sources.draft && !src.available_sources.latest_release;
}

async function confirmIfDirty(): Promise<boolean> {
  if (!isDirty.value) return true;
  try {
    await ElMessageBox.confirm(
      t("drafts.navigate.prompt"),
      t("drafts.navigate.title"),
      {
        confirmButtonText: t("drafts.navigate.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
    return true;
  } catch {
    return false;
  }
}

async function switchConfigFile(cfId: number) {
  if (!(await confirmIfDirty())) return;
  router.push({
    name: ROUTE_NAMES.DRAFT_EDITOR,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
      configFileId: String(cfId),
    },
  });
}

async function loadDraftResources() {
  const did = deploymentId.value;
  const cid = configFileId.value;
  if (Number.isNaN(did) || Number.isNaN(cid)) return;

  resourceLoading.value = true;
  resourceError.value = null;
  deployment.value = null;
  configFile.value = null;
  draft.value = null;
  content.value = "";
  savedContent.value = "";
  draftWasMissing.value = false;
  savedVersions.value = [];
  savedVersionsError.value = null;
  selectedSavedVersionId.value = null;
  savedVersionDetail.value = null;
  savedVersionNote.value = "";
  savedNoteSnapshot.value = "";

  try {
    const [deploymentResult, configResult, configListResult] =
      await Promise.all([
        deploymentInstancesApi.getDeploymentInstance(did),
        configFilesApi.getConfigFile(cid),
        configFilesApi.listConfigFiles({
          project_id: Number(route.params.projectId),
          status: "active",
        }),
      ]);
    deployment.value = deploymentResult;
    configFile.value = configResult;
    configFiles.value = configListResult.items;

    if (
      deploymentResult.project_id !== projectId.value ||
      configResult.project_id !== projectId.value
    ) {
      resourceError.value = new ApiRequestError(404, {
        code: "resource_not_found",
        message: "resource not found",
      });
      return;
    }

    if (!canEdit.value) {
      return;
    }

    // Load preview status for config switcher badges (non-blocking)
    loadPreviewStatus(did);
    if (canViewSavedVersions.value) {
      await loadSavedVersions({ keepSelection: false });
    }

    try {
      const draftResult = await draftsApi.getDraft(did, cid);
      applyDraft(draftResult);
    } catch (err) {
      if (err instanceof ApiRequestError && err.code === "draft_not_found") {
        draftWasMissing.value = true;
        content.value = "";
        savedContent.value = "";
        return;
      }
      throw err;
    }
  } catch (err) {
    if (err instanceof ApiRequestError) {
      resourceError.value = err;
    } else {
      resourceError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("drafts.page.loadError"),
      });
    }
  } finally {
    resourceLoading.value = false;
  }
}

async function loadPreviewStatus(did: number) {
  try {
    const preview = await deploymentInstancesApi.previewDeploymentBundle(did);
    const map: Record<number, string> = {};
    for (const item of preview.items) {
      if (
        item.status === "missing_required" ||
        item.status === "missing_optional"
      ) {
        map[item.config_file_id] = item.status;
      } else {
        map[item.config_file_id] = item.source;
      }
    }
    previewStatusMap.value = map;
  } catch {
    // Non-critical; silently ignore
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  await fetchProject(pid);
  await loadDraftResources();
}

function applyDraft(value: DraftResponse) {
  draft.value = value;
  draftWasMissing.value = false;
  content.value = value.content;
  savedContent.value = value.content;
}

async function loadSavedVersions(options?: { keepSelection?: boolean }) {
  if (!canViewSavedVersions.value) {
    return;
  }

  savedVersionsLoading.value = true;
  savedVersionsError.value = null;
  try {
    const result = await savedVersionsApi.listSavedVersions({
      deployment_instance_id: deploymentId.value,
      config_file_id: configFileId.value,
    });
    savedVersions.value = result.items;

    if (result.items.length === 0) {
      selectedSavedVersionId.value = null;
      savedVersionDetail.value = null;
      savedVersionNote.value = "";
      return;
    }

    const keepSelection = options?.keepSelection ?? true;
    const keepCurrent =
      keepSelection &&
      selectedSavedVersionId.value !== null &&
      result.items.some((item) => item.id === selectedSavedVersionId.value);

    const nextSelectedId = keepCurrent
      ? selectedSavedVersionId.value
      : result.items[0].id;
    if (nextSelectedId !== null) {
      await selectSavedVersion(nextSelectedId);
    }
  } catch (err) {
    if (err instanceof ApiRequestError) {
      savedVersionsError.value = err;
    } else {
      savedVersionsError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("savedVersions.error.loadList"),
      });
    }
  } finally {
    savedVersionsLoading.value = false;
  }
}

async function confirmIfNoteDirty(): Promise<boolean> {
  if (!isNoteDirty.value) return true;
  try {
    await ElMessageBox.confirm(
      t("savedVersions.note.discardPrompt"),
      t("savedVersions.note.discardTitle"),
      {
        confirmButtonText: t("savedVersions.note.discardConfirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
    return true;
  } catch {
    return false;
  }
}

async function selectSavedVersion(id: number) {
  if (!(await confirmIfNoteDirty())) return;
  // Skip re-fetch if same item is already loaded and note is clean
  if (
    selectedSavedVersionId.value === id &&
    savedVersionDetail.value?.id === id
  )
    return;
  selectedSavedVersionId.value = id;
  savedVersionDetailLoading.value = true;
  try {
    const result = await savedVersionsApi.getSavedVersion(id);
    // Stale-response guard: discard if user already clicked another item
    if (selectedSavedVersionId.value !== id) return;
    savedVersionDetail.value = result.saved_version;
    savedVersionNote.value = result.saved_version.note ?? "";
    savedNoteSnapshot.value = savedVersionNote.value;
  } catch (err) {
    if (selectedSavedVersionId.value !== id) return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    if (selectedSavedVersionId.value === id) {
      savedVersionDetailLoading.value = false;
    }
  }
}

async function handleUpdateSavedVersionNote() {
  if (!savedVersionDetail.value) {
    return;
  }

  const note = savedVersionNote.value.trim();
  if (note.length > SAVED_VERSION_NOTE_MAX_LENGTH) {
    ElMessage.error(t("savedVersions.error.noteTooLong"));
    return;
  }

  updatingSavedVersionNote.value = true;
  try {
    const result = await savedVersionsApi.updateSavedVersion(
      savedVersionDetail.value.id,
      {
        note: note.length > 0 ? note : null,
      },
    );
    savedVersionDetail.value = result.saved_version;
    savedVersionNote.value = result.saved_version.note ?? "";
    savedNoteSnapshot.value = savedVersionNote.value;
    savedVersions.value = savedVersions.value.map((item) => {
      if (item.id !== result.saved_version.id) {
        return item;
      }
      return {
        ...item,
        note: result.saved_version.note,
      };
    });
    ElMessage.success(t("toast.savedVersions.noteSaved"));
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    updatingSavedVersionNote.value = false;
  }
}

async function handleRestoreSavedVersion() {
  if (!savedVersionDetail.value) {
    return;
  }

  if (isDirty.value) {
    try {
      await ElMessageBox.confirm(
        t("drafts.navigate.prompt"),
        t("drafts.navigate.title"),
        {
          confirmButtonText: t("drafts.navigate.confirm"),
          cancelButtonText: t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }
  }

  try {
    await ElMessageBox.confirm(
      t("savedVersions.restore.prompt"),
      t("savedVersions.restore.title"),
      {
        confirmButtonText: t("savedVersions.restore.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  restoringFromSavedVersion.value = true;
  try {
    const result = await savedVersionsApi.restoreSavedVersion(
      savedVersionDetail.value.id,
      {
        base_version: draft.value?.version ?? null,
      },
    );
    applyDraft(result.draft);
    ElMessage.success(t("toast.savedVersions.restored"));
    loadPreviewStatus(deploymentId.value);
    await loadSavedVersions();
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    restoringFromSavedVersion.value = false;
  }
}

async function handleDeleteSavedVersion() {
  if (!savedVersionDetail.value) {
    return;
  }

  try {
    await ElMessageBox.confirm(
      t("savedVersions.delete.prompt"),
      t("savedVersions.delete.title"),
      {
        confirmButtonText: t("savedVersions.delete.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  deletingSavedVersion.value = true;
  try {
    await savedVersionsApi.deleteSavedVersion(savedVersionDetail.value.id);
    ElMessage.success(t("toast.savedVersions.deleted"));
    await loadSavedVersions({ keepSelection: false });
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    deletingSavedVersion.value = false;
  }
}

async function handleSave() {
  if (!configFile.value) return;

  saving.value = true;
  try {
    const result = await draftsApi.updateDraft(
      deploymentId.value,
      configFileId.value,
      {
        content: content.value,
        format: configFile.value.format,
        base_version: draft.value?.version ?? 0,
      },
    );
    applyDraft(result);
    await loadSavedVersions({ keepSelection: false });
    ElMessage.success(t("toast.drafts.saved"));
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    saving.value = false;
  }
}

async function handleDiscard() {
  try {
    await ElMessageBox.confirm(
      t("drafts.discard.prompt"),
      t("drafts.discard.title"),
      {
        confirmButtonText: t("drafts.discard.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  discarding.value = true;
  try {
    await draftsApi.deleteDraft(deploymentId.value, configFileId.value);
    draft.value = null;
    draftWasMissing.value = true;

    // Load fallback content so the editor shows what the instance uses now
    // instead of a misleading empty state.
    try {
      const preview = await deploymentInstancesApi.previewDeploymentBundle(
        deploymentId.value,
      );
      const item = preview.items.find(
        (i) => i.config_file_id === configFileId.value,
      );
      content.value = item?.content ?? "";
      savedContent.value = content.value;
    } catch {
      content.value = "";
      savedContent.value = "";
    }

    ElMessage.success(t("toast.drafts.discarded"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    discarding.value = false;
  }
}

async function handleRestoreFromRelease() {
  try {
    await ElMessageBox.confirm(
      t("drafts.restore.prompt"),
      t("drafts.restore.title"),
      {
        confirmButtonText: t("drafts.restore.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  restoring.value = true;
  try {
    const result = await draftsApi.cloneDraft(
      deploymentId.value,
      configFileId.value,
      {
        source_deployment_instance_id: deploymentId.value,
        source_kind: "latest_release",
      },
    );
    applyDraft(result.draft);
    ElMessage.success(t("toast.drafts.restoredFromRelease"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    restoring.value = false;
  }
}

async function handlePublish() {
  if (!draft.value || isDirty.value) {
    ElMessage.warning(t("drafts.notice.saveBeforePublish"));
    return;
  }

  try {
    const { value } = await ElMessageBox.prompt(
      t("drafts.publish.prompt"),
      t("drafts.publish.title"),
      {
        inputType: "textarea",
        inputPlaceholder: t("drafts.publish.placeholder"),
        confirmButtonText: t("common.publish"),
        cancelButtonText: t("common.cancel"),
      },
    );
    publishing.value = true;
    const release = await releasesApi.publishRelease({
      project_id: projectId.value,
      deployment_instance_id: deploymentId.value,
      config_file_id: configFileId.value,
      change_summary: value || null,
    });
    ElMessage.success(
      t("toast.releases.published", { revision: release.revision }),
    );
    router.push({
      name: ROUTE_NAMES.RELEASE_DETAIL,
      params: { projectId: route.params.projectId, releaseId: release.id },
    });
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    publishing.value = false;
  }
}

// Clone from other instance
const CLONE_ERROR_KEYS: Record<string, string> = {
  draft_not_found: "drafts.cloneDialog.error.sourceDraftNotFound",
  release_not_found: "drafts.cloneDialog.error.sourceReleaseNotFound",
  draft_validation_failed: "drafts.cloneDialog.error.validationFailed",
};

async function searchCloneSources(keyword?: string) {
  const seq = ++cloneSearchSeq;
  const normalizedKeyword = keyword || undefined;
  cloneSearchKeyword.value = normalizedKeyword;
  cloneInstancesLoading.value = true;
  cloneLoadError.value = false;
  cloneNextCursor.value = null;
  try {
    const result = await cloneSourcesApi.listCloneSources({
      project_id: projectId.value,
      target_deployment_id: deploymentId.value,
      config_file_id: configFileId.value,
      keyword: normalizedKeyword,
      limit: 50,
    });
    if (seq !== cloneSearchSeq) return;
    cloneSources.value = result.items;
    cloneNextCursor.value = result.next_cursor;
  } catch {
    if (seq !== cloneSearchSeq) return;
    cloneSources.value = [];
    cloneLoadError.value = true;
  } finally {
    if (seq === cloneSearchSeq) {
      cloneInstancesLoading.value = false;
    }
  }
}

async function loadMoreCloneSources() {
  if (!cloneNextCursor.value || cloneLoadingMore.value) return;
  const seq = cloneSearchSeq;
  cloneLoadingMore.value = true;
  try {
    const result = await cloneSourcesApi.listCloneSources({
      project_id: projectId.value,
      target_deployment_id: deploymentId.value,
      config_file_id: configFileId.value,
      keyword: cloneSearchKeyword.value,
      limit: 50,
      cursor: cloneNextCursor.value,
    });
    if (seq !== cloneSearchSeq) return;
    cloneSources.value = [...cloneSources.value, ...result.items];
    cloneNextCursor.value = result.next_cursor;
  } catch {
    // silently ignore load-more errors; user can retry
  } finally {
    if (seq === cloneSearchSeq) {
      cloneLoadingMore.value = false;
    }
  }
}

function handleCloneRemoteSearch(keyword: string) {
  if (cloneSearchTimer) globalThis.clearTimeout(cloneSearchTimer);
  if (cloneLoadingMore.value) return;
  cloneSearchTimer = globalThis.setTimeout(() => {
    searchCloneSources(keyword);
  }, 300);
}

watch(cloneDialogVisible, (visible) => {
  if (visible) {
    cloneSourceInstanceId.value = null;
    cloneSourceKind.value = "draft";
    cloneLoadError.value = false;
    cloneSources.value = [];
    cloneNextCursor.value = null;
    cloneSearchKeyword.value = undefined;
    searchCloneSources();
  }
});

// Auto-select best available source kind when instance changes
watch(selectedCloneSource, (src) => {
  if (!src) return;
  if (src.is_template || !src.available_sources.latest_release) {
    cloneSourceKind.value = "draft";
  }
});

async function handleCloneFromInstance() {
  if (!cloneSourceInstanceId.value) return;

  if (isDirty.value) {
    try {
      await ElMessageBox.confirm(
        t("drafts.cloneDialog.overwritePrompt"),
        t("drafts.cloneDialog.overwriteTitle"),
        {
          confirmButtonText: t("drafts.cloneDialog.overwriteConfirm"),
          cancelButtonText: t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }
  }

  cloning.value = true;
  try {
    const result = await draftsApi.cloneDraft(
      deploymentId.value,
      configFileId.value,
      {
        source_deployment_instance_id: cloneSourceInstanceId.value,
        source_kind: cloneSourceKind.value,
      },
    );
    applyDraft(result.draft);
    cloneDialogVisible.value = false;
    ElMessage.success(t("toast.drafts.clonedFromInstance"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      const cloneErrorKey = CLONE_ERROR_KEYS[err.code];
      ElMessage.error(
        cloneErrorKey
          ? t(cloneErrorKey)
          : getErrorMessage(err.code, err.message),
      );
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    cloning.value = false;
  }
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
  () => [
    route.params.projectId,
    route.params.deploymentId,
    route.params.configFileId,
  ],
  () => loadAll(),
);

// ---- Global route-leave & browser-close guards ----

onBeforeRouteLeave(async () => {
  if (!isDirty.value) return true;
  try {
    await ElMessageBox.confirm(
      t("drafts.navigate.prompt"),
      t("drafts.navigate.title"),
      {
        confirmButtonText: t("drafts.navigate.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
    return true;
  } catch {
    return false;
  }
});

function onBeforeUnloadHandler(e: {
  preventDefault: () => void;
  returnValue?: string;
}) {
  if (isDirty.value) {
    e.preventDefault();
    e.returnValue = "";
  }
}

onMounted(() => {
  globalThis.addEventListener("beforeunload", onBeforeUnloadHandler);
});
onBeforeUnmount(() => {
  if (cloneSearchTimer) {
    globalThis.clearTimeout(cloneSearchTimer);
  }
  globalThis.removeEventListener("beforeunload", onBeforeUnloadHandler);
});
</script>

<style scoped>
.draft-editor-page {
  width: 100%;
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

.draft-editor-page__draft-actions {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__meta,
.draft-editor-page__notice {
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__code {
  font-family: monospace;
}
</style>
