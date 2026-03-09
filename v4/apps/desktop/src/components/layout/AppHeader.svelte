<script lang="ts">
  import AppToolbar from "../AppToolbar.svelte";

  let {
    searchValue,
    onsearch,
    onclearsearch,
    selectedCount,
    selectedRamMB,
    grouping,
    totalFindings,
    aiLoading,
    aiProfile,
    fontSize,
    onselectall,
    onselectnone,
    onkillselected,
    ontogglegrouping,
    onchangepofile,
    onanalyze,
    onopensecurity,
    ontoggledashboard,
    dashboardCollapsed,
    ontoggleautomations,
    onopenplugins,
    onopensettings,
    onopenhelp,
    ondecreasefont,
    onincreasefont
  } = $props();
</script>

<div class="app-header-container">
  <AppToolbar
    searchValue={searchValue}
    onsearch={onsearch}
    onclearsearch={onclearsearch}
    selectedCount={selectedCount}
    selectedRamMB={selectedRamMB}
    grouping={grouping}
    totalFindings={totalFindings}
    aiLoading={aiLoading}
    aiProfile={aiProfile}
    fontSize={fontSize}
    onselectall={onselectall}
    onselectnone={onselectnone}
    onkillselected={onkillselected}
    ontogglegrouping={ontogglegrouping}
    onchangepofile={onchangepofile}
    onanalyze={onanalyze}
    onopensecurity={onopensecurity}
    ontoggledashboard={ontoggledashboard}
    dashboardCollapsed={dashboardCollapsed}
    ontoggleautomations={ontoggleautomations}
    onopenplugins={onopenplugins}
    onopensettings={onopensettings}
    onopenhelp={onopenhelp}
    ondecreasefont={ondecreasefont}
    onincreasefont={onincreasefont}
  />
</div>

<style>
  .app-header-container {
    flex-shrink: 0;
  }
</style>
