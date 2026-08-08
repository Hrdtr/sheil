<script setup lang="ts">
import type { SidebarProps } from '@/components/ui/sidebar';
import {
  EllipsisVerticalIcon,
  EditIcon,
  MonitorIcon,
  PlusIcon,
  SearchIcon,
  TrashIcon,
} from '@lucide/vue';
import { SettingsIcon } from '@lucide/vue';

const props = withDefaults(defineProps<SidebarProps>(), {
  variant: 'inset',
});

const { groupedHosts } = useHosts();
const { connect } = useSessions();
const { openSettings } = useSettings();

const openQuickConnectDialog = inject<() => void>('openQuickConnectDialog');
</script>

<template>
  <Hosts v-slot="{ createHost, updateHost, deleteHost }">
    <Sidebar v-bind="props" data-tauri-drag-region class="p-0">
      <SidebarHeader class="pt-[calc(--spacing(2)+1px)] md:pt-3" data-tauri-drag-region>
        <SidebarGroup data-tauri-drag-region>
          <SidebarGroupContent class="flex flex-row items-center gap-2" data-tauri-drag-region>
            <div class="relative flex-1" data-tauri-drag-region></div>
            <div class="shrink-0 flex flex-row items-center gap-0.5" data-tauri-drag-region>
              <Button
                variant="secondary"
                size="icon-sm"
                class="shrink-0 rounded-lg"
                @click="openQuickConnectDialog"
              >
                <SearchIcon class="size-4" />
              </Button>
              <Button
                variant="secondary"
                size="icon-sm"
                class="shrink-0 rounded-lg"
                @click="createHost"
              >
                <PlusIcon class="size-4.5" />
              </Button>
            </div>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarHeader>
      <SidebarContent data-tauri-drag-region>
        <div
          v-if="groupedHosts.length === 0"
          class="flex flex-col justify-center gap-2 px-6 py-8"
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
                  <SidebarMenuButton
                    size="lg"
                    class="hover:bg-accent dark:hover:bg-accent/50 transition-colors rounded-lg h-fit group/button px-2.5 py-[5.5px]"
                    @click="() => connect(host.id)"
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
                        <DropdownMenuItem variant="destructive" @click="() => deleteHost(host.id)">
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
      </SidebarContent>
      <SidebarFooter data-tauri-drag-region class="pb-[calc(--spacing(2)+1px)] md:pb-2">
        <div class="flex items-center gap-0.5 p-2" data-tauri-drag-region>
          <Button
            variant="ghost"
            size="sm"
            class="flex-1 justify-start font-normal rounded-lg pr-1.5"
            @click="openSettings"
          >
            <span>Settings</span>
          </Button>
          <ColorModeToggle variant="secondary" size="icon-sm" class="rounded-lg" />
        </div>
      </SidebarFooter>
    </Sidebar>
  </Hosts>
</template>
