<script setup lang="ts"></script>

<template>
  <div class="flex flex-col gap-10 w-full h-full overflow-y-auto px-4 pt-4 pb-4 @container">
    <div class="columns-1 @2xl:columns-2 @5xl:columns-3 gap-4 w-full max-w-screen-2xl mx-auto">
      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">General</h3>
          <SettingsGeneral />
        </div>
      </div>

      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">Terminal</h3>
          <SettingsTerminal />
        </div>
      </div>

      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">SSH</h3>
          <SettingsSsh />
        </div>
      </div>

      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">Data</h3>
          <SettingsData />
        </div>
      </div>

      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">AI</h3>
          <SettingsAi />
        </div>
      </div>

      <div class="break-inside-avoid pb-4">
        <div class="flex flex-col gap-4 px-6 py-5 bg-sidebar rounded-xl">
          <h3 class="text-xl font-thin">Sheil</h3>
          <SettingsAbout />
        </div>
      </div>
    </div>
  </div>
</template>
