<script setup lang="ts"></script>

<template>
  <div class="flex flex-col gap-10 w-full h-full overflow-y-auto px-4 pt-4 pb-4 @container">
    <div class="columns-1 @2xl:columns-2 @5xl:columns-3 w-full max-w-screen-2xl mx-auto">
      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">General</h3>
        <SettingsGeneral />
      </section>

      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">Terminal</h3>
        <SettingsTerminal />
      </section>

      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">SSH</h3>
        <SettingsSsh />
      </section>

      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">Data</h3>
        <SettingsData />
      </section>

      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">AI</h3>
        <SettingsAi />
      </section>

      <section class="flex flex-col gap-4 break-inside-avoid px-4 pb-10">
        <h3 class="text-xl font-thin">Sheil</h3>
        <SettingsAbout />
      </section>
    </div>
  </div>
</template>
