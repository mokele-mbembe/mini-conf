import type { App, Plugin } from "vue";
import { ElAlert } from "element-plus/es/components/alert/index";
import { ElButton } from "element-plus/es/components/button/index";
import { ElCard } from "element-plus/es/components/card/index";
import { ElCheckbox } from "element-plus/es/components/checkbox/index";
import {
  ElAside,
  ElContainer,
  ElHeader,
  ElMain,
} from "element-plus/es/components/container/index";
import {
  ElDescriptions,
  ElDescriptionsItem,
} from "element-plus/es/components/descriptions/index";
import { ElDialog } from "element-plus/es/components/dialog/index";
import { ElDivider } from "element-plus/es/components/divider/index";
import { ElDrawer } from "element-plus/es/components/drawer/index";
import {
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
} from "element-plus/es/components/dropdown/index";
import { ElEmpty } from "element-plus/es/components/empty/index";
import { ElForm, ElFormItem } from "element-plus/es/components/form/index";
import { ElIcon } from "element-plus/es/components/icon/index";
import { ElInput } from "element-plus/es/components/input/index";
import { ElInputNumber } from "element-plus/es/components/input-number/index";
import { ElLoading } from "element-plus/es/components/loading/index";
import { ElPagination } from "element-plus/es/components/pagination/index";
import {
  ElRadio,
  ElRadioButton,
  ElRadioGroup,
} from "element-plus/es/components/radio/index";
import { ElResult } from "element-plus/es/components/result/index";
import { ElOption, ElSelect } from "element-plus/es/components/select/index";
import { ElSkeleton } from "element-plus/es/components/skeleton/index";
import { ElSpace } from "element-plus/es/components/space/index";
import { ElStep, ElSteps } from "element-plus/es/components/steps/index";
import { ElSwitch } from "element-plus/es/components/switch/index";
import { ElTabPane, ElTabs } from "element-plus/es/components/tabs/index";
import { ElTable, ElTableColumn } from "element-plus/es/components/table/index";
import { ElTag } from "element-plus/es/components/tag/index";
import { ElTooltip } from "element-plus/es/components/tooltip/index";

import "element-plus/es/components/alert/style/css";
import "element-plus/es/components/aside/style/css";
import "element-plus/es/components/button/style/css";
import "element-plus/es/components/card/style/css";
import "element-plus/es/components/checkbox/style/css";
import "element-plus/es/components/container/style/css";
import "element-plus/es/components/descriptions/style/css";
import "element-plus/es/components/descriptions-item/style/css";
import "element-plus/es/components/dialog/style/css";
import "element-plus/es/components/divider/style/css";
import "element-plus/es/components/drawer/style/css";
import "element-plus/es/components/dropdown/style/css";
import "element-plus/es/components/dropdown-item/style/css";
import "element-plus/es/components/dropdown-menu/style/css";
import "element-plus/es/components/empty/style/css";
import "element-plus/es/components/form/style/css";
import "element-plus/es/components/form-item/style/css";
import "element-plus/es/components/header/style/css";
import "element-plus/es/components/icon/style/css";
import "element-plus/es/components/input/style/css";
import "element-plus/es/components/input-number/style/css";
import "element-plus/es/components/loading/style/css";
import "element-plus/es/components/main/style/css";
import "element-plus/es/components/message/style/css";
import "element-plus/es/components/message-box/style/css";
import "element-plus/es/components/option/style/css";
import "element-plus/es/components/pagination/style/css";
import "element-plus/es/components/radio/style/css";
import "element-plus/es/components/radio-button/style/css";
import "element-plus/es/components/radio-group/style/css";
import "element-plus/es/components/result/style/css";
import "element-plus/es/components/select/style/css";
import "element-plus/es/components/skeleton/style/css";
import "element-plus/es/components/space/style/css";
import "element-plus/es/components/step/style/css";
import "element-plus/es/components/steps/style/css";
import "element-plus/es/components/switch/style/css";
import "element-plus/es/components/tab-pane/style/css";
import "element-plus/es/components/table/style/css";
import "element-plus/es/components/table-column/style/css";
import "element-plus/es/components/tabs/style/css";
import "element-plus/es/components/tag/style/css";
import "element-plus/es/components/tooltip/style/css";

const components: Plugin[] = [
  ElAlert,
  ElAside,
  ElButton,
  ElCard,
  ElCheckbox,
  ElContainer,
  ElDescriptions,
  ElDescriptionsItem,
  ElDialog,
  ElDivider,
  ElDrawer,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElHeader,
  ElIcon,
  ElInput,
  ElInputNumber,
  ElMain,
  ElOption,
  ElPagination,
  ElRadio,
  ElRadioButton,
  ElRadioGroup,
  ElResult,
  ElSelect,
  ElSkeleton,
  ElSpace,
  ElStep,
  ElSteps,
  ElSwitch,
  ElTabPane,
  ElTable,
  ElTableColumn,
  ElTabs,
  ElTag,
  ElTooltip,
];

export function setupElementPlus(app: App): void {
  for (const component of components) {
    app.use(component);
  }
  app.use(ElLoading);
}
