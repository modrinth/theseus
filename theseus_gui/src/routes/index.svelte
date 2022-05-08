<script lang="ts">
import Instance from "$components/Instance.svelte";
import CardRow from "$components/CardRow.svelte";
</script>

<CardRow title="Jump back in">
    {#each Array(5) as _, i}
        <Instance title="New Caves" id="234" version="1.18" image="https://i.ibb.co/8KDxBwq/patchnotes-cavesandcliffs.jpg" />
    {/each}
</CardRow>

<CardRow title="Popular packs">
    {#each Array(5) as _, i}
        <Instance title="All of Fabric 5" id="567" version="1.18.1" image="https://media.forgecdn.net/avatars/458/829/637733746768258525.png" modpack />
    {/each}
</CardRow>

<CardRow title="New releases">
    {#each Array(5) as _, i}
        <Instance title="New Caves" id="234" version="1.18.2" image="https://i.ibb.co/8KDxBwq/patchnotes-cavesandcliffs.jpg" />
    {/each}
</CardRow>

<style lang="postcss">

</style>
