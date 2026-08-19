<script setup lang="ts">
import {
  BracesIcon,
  EllipsisVerticalIcon,
  EditIcon,
  FileCodeIcon,
  KeyIcon,
  LockIcon,
  MonitorIcon,
  PlusIcon,
  SettingsIcon,
  SquareTerminalIcon,
  TrashIcon,
  PlugZapIcon,
} from '@lucide/vue';
import { platform } from '@tauri-apps/plugin-os';
import { useSidebar, type SidebarProps } from '@/components/ui/sidebar';

type SidebarView = 'hosts' | 'credentials' | 'snippets';

const props = withDefaults(defineProps<SidebarProps>(), {
  variant: 'inset',
});

const { groupedHosts } = useHosts();
const { connect, focusOrConnect } = useSessions();
const { openSettings, settingsActive } = useSettingsTab();
const { setOpenMobile } = useSidebar();

const openQuickConnectDialog = inject<() => void>('openQuickConnectDialog');

const isMacos = platform() === 'macos';
const activeView = ref<SidebarView>('hosts');

const credentialsPanelRef = useTemplateRef('credentialsPanel');
const snippetsPanelRef = useTemplateRef('snippetsPanel');
</script>

<template>
  <Hosts v-slot="{ createHost, updateHost, deleteHost }">
    <Sidebar v-bind="props" data-tauri-drag-region class="p-0">
      <div class="flex h-full min-h-0" data-tauri-drag-region>
        <!-- Vertical rail: switches the main list -->
        <div
          class="shrink-0 ml-4 flex flex-col gap-1 pt-4.5 pb-4"
          data-tauri-drag-region
          :class="isMacos ? 'pt-13' : ''"
        >
          <div
            class="flex-1 flex flex-col items-center gap-1 overflow-y-auto no-scrollbar"
            data-tauri-drag-region
          >
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon"
                  class="rounded-lg"
                  :class="
                    activeView === 'hosts'
                      ? 'bg-accent text-accent-foreground'
                      : 'text-muted-foreground'
                  "
                  aria-label="Hosts"
                  @click="activeView = 'hosts'"
                >
                  <MonitorIcon class="size-4.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>Hosts</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon"
                  class="rounded-lg"
                  :class="
                    activeView === 'snippets'
                      ? 'bg-accent text-accent-foreground'
                      : 'text-muted-foreground'
                  "
                  aria-label="Snippets"
                  @click="activeView = 'snippets'"
                >
                  <BracesIcon class="size-4.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>Snippets</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon"
                  class="rounded-lg"
                  :class="
                    activeView === 'credentials'
                      ? 'bg-accent text-accent-foreground'
                      : 'text-muted-foreground'
                  "
                  aria-label="Credentials"
                  @click="activeView = 'credentials'"
                >
                  <KeyIcon class="size-4.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>Credentials</p>
              </TooltipContent>
            </Tooltip>
          </div>

          <div class="flex flex-col items-center gap-1" data-tauri-drag-region>
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="icon"
                  class="rounded-lg"
                  :class="settingsActive ? 'text-foreground' : 'text-muted-foreground'"
                  aria-label="Settings"
                  @click="openSettings"
                >
                  <SettingsIcon class="size-4.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>Settings</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        <!-- Main column -->
        <div class="flex min-w-0 flex-1 flex-col" data-tauri-drag-region>
          <SidebarHeader
            class="pt-[calc(--spacing(2)+1px)] md:pt-3 **:data-[slot='sidebar-group']:pl-0! pb-0"
            data-tauri-drag-region
          >
            <SidebarGroup data-tauri-drag-region>
              <SidebarGroupContent class="flex flex-row items-center gap-2" data-tauri-drag-region>
                <div class="relative flex-1" data-tauri-drag-region></div>
                <div class="shrink-0 flex flex-row items-center gap-1" data-tauri-drag-region>
                  <Button
                    variant="secondary"
                    size="icon-sm"
                    class="shrink-0 rounded-lg"
                    @click="
                      () => {
                        setOpenMobile(false);
                        openQuickConnectDialog?.();
                      }
                    "
                  >
                    <PlugZapIcon class="size-4" />
                  </Button>

                  <Button
                    v-if="activeView === 'hosts'"
                    variant="secondary"
                    size="icon-sm"
                    class="shrink-0 rounded-lg"
                    @click="createHost"
                  >
                    <PlusIcon class="size-4.5" />
                  </Button>

                  <DropdownMenu v-else-if="activeView === 'credentials'">
                    <DropdownMenuTrigger as-child>
                      <Button variant="secondary" size="icon-sm" class="shrink-0 rounded-lg">
                        <PlusIcon class="size-4.5" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent class="w-fit">
                      <DropdownMenuGroup>
                        <DropdownMenuItem @click="credentialsPanelRef?.openAddPassword()">
                          <LockIcon class="size-3.5" />
                          <span>Add Password</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem @click="credentialsPanelRef?.openImport()">
                          <KeyIcon class="size-3.5" />
                          <span>Import SSH Key</span>
                        </DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>

                  <DropdownMenu v-else>
                    <DropdownMenuTrigger as-child>
                      <Button variant="secondary" size="icon-sm" class="shrink-0 rounded-lg">
                        <PlusIcon class="size-4.5" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent class="w-fit">
                      <DropdownMenuGroup>
                        <DropdownMenuItem @click="snippetsPanelRef?.openAdd()">
                          <SquareTerminalIcon class="size-3.5" />
                          <span>New Snippet</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem @click="snippetsPanelRef?.openTemplates()">
                          <FileCodeIcon class="size-3.5" />
                          <span>From Template…</span>
                        </DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </SidebarGroupContent>
            </SidebarGroup>
          </SidebarHeader>

          <TooltipProvider :delay-duration="1250">
            <SidebarContent data-tauri-drag-region class="**:data-[slot='sidebar-group']:pl-0!">
              <AppSidebarCredentialsPanel
                v-if="activeView === 'credentials'"
                ref="credentialsPanel"
                data-tauri-drag-region
              />

              <AppSidebarSnippetsPanel
                v-else-if="activeView === 'snippets'"
                ref="snippetsPanel"
                data-tauri-drag-region
              />

              <template v-else>
                <div
                  v-if="groupedHosts.length === 0"
                  class="flex flex-col justify-center gap-2 px-4 py-1.5"
                  data-tauri-drag-region
                >
                  <div class="space-y-1" data-tauri-drag-region>
                    <p class="text-sm font-medium" data-tauri-drag-region>Add your first host</p>
                    <p class="text-xs text-muted-foreground leading-normal" data-tauri-drag-region>
                      Save SSH connections to quickly access your servers
                    </p>
                  </div>
                  <div data-tauri-drag-region>
                    <Button variant="outline" size="sm" class="mt-1" @click="createHost">
                      <PlusIcon class="size-3.5" />
                      Add Host
                    </Button>
                  </div>
                </div>
                <div v-else data-tauri-drag-region>
                  <SidebarGroup class="py-0 sticky top-0 z-10 bg-sidebar" data-tauri-drag-region>
                    <SidebarGroupLabel
                      class="px-4.5 text-sm text-sidebar-foreground"
                      data-tauri-drag-region
                      >Hosts</SidebarGroupLabel
                    >
                  </SidebarGroup>
                  <SidebarGroup
                    v-for="[groupName, groupHosts] in groupedHosts"
                    :key="groupName"
                    data-tauri-drag-region
                  >
                    <SidebarGroupLabel class="px-4.5" data-tauri-drag-region>{{
                      groupName
                    }}</SidebarGroupLabel>
                    <SidebarGroupContent data-tauri-drag-region>
                      <SidebarMenu class="gap-0.5" data-tauri-drag-region>
                        <SidebarMenuItem
                          v-for="host in groupHosts"
                          :key="host.id"
                          class="mx-2 group/item"
                          data-tauri-drag-region
                        >
                          <Tooltip>
                            <TooltipTrigger as-child>
                              <SidebarMenuButton
                                size="lg"
                                class="hover:bg-accent dark:hover:bg-accent/50 transition-colors rounded-lg h-fit group/button px-2.5 py-[5.5px]"
                                @click="() => focusOrConnect(host.id)"
                                @dblclick="() => connect(host.id)"
                              >
                                <div
                                  class="flex flex-col gap-0.5 min-w-0 text-muted-foreground group-hover/button:text-foreground transition-colors"
                                >
                                  <span class="truncate">{{ host.name }}</span>
                                  <span
                                    class="truncate text-xs text-muted-foreground -mt-4 group-hover/button:mt-0 opacity-0 group-hover/button:opacity-100 transition-all duration-200 ease-in-out"
                                  >
                                    {{ host.username }}@{{ host.host }}
                                  </span>
                                </div>
                              </SidebarMenuButton>
                            </TooltipTrigger>
                            <TooltipContent as="div" class="flex flex-col items-start gap-0.5">
                              <span class="text-xs">{{ host.name }}</span>
                              <span class="text-xs opacity-50"
                                >{{ host.username }}@{{ host.host }}</span
                              >
                            </TooltipContent>
                          </Tooltip>
                          <DropdownMenu>
                            <DropdownMenuTrigger as-child>
                              <SidebarMenuAction
                                class="-mt-1 opacity-50 group-hover/item:opacity-100 transition-opacity"
                              >
                                <EllipsisVerticalIcon />
                              </SidebarMenuAction>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent>
                              <DropdownMenuGroup>
                                <DropdownMenuItem @click="() => updateHost(host)">
                                  <EditIcon class="size-3.5" />
                                  <span>Edit</span>
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                  variant="destructive"
                                  @click="() => deleteHost(host.id)"
                                >
                                  <TrashIcon class="size-3.5" />
                                  <span>Delete</span>
                                </DropdownMenuItem>
                              </DropdownMenuGroup>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </SidebarMenuItem>
                      </SidebarMenu>
                    </SidebarGroupContent>
                  </SidebarGroup>
                </div>
              </template>
            </SidebarContent>
          </TooltipProvider>

          <SidebarFooter data-tauri-drag-region class="pb-[calc(--spacing(2)+1px)] md:pb-2">
          </SidebarFooter>
        </div>
      </div>
    </Sidebar>
  </Hosts>
</template>
