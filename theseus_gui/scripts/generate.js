import fetch from 'node-fetch';
import { promises as fs } from 'fs';

const API_URL =
  process.env.VITE_API_URL || process.env?.NODE_ENV === 'development'
    ? 'https://staging-api.modrinth.com/v2/'
    : 'https://api.modrinth.com/v2/';
const GENERATED_PATH = './generated/';

(async () => {
  /* GAME VERSIONS */

  // Fetch data
  let gameVersions = await (await fetch(API_URL + 'tag/game_version')).json();

  // Write JSON file
  await fs.writeFile(GENERATED_PATH + 'gameVersions.json', JSON.stringify(gameVersions));

  console.log('Generated gameVersions.json');

  /* TAGS */

  // Fetch data
  let [categories, loaders, licenses, donationPlatforms] = await Promise.all([
    await (await fetch(API_URL + 'tag/category')).json(),
    await (await fetch(API_URL + 'tag/loader')).json(),
    await (await fetch(API_URL + 'tag/license')).json(),
    await (await fetch(API_URL + 'tag/donation_platform')).json(),
  ]);

  // Create single object with icons
  let tagIcons = {
    ...categories.reduce((a, v) => ({ ...a, [v.name]: v.icon }), {}),
    ...loaders.reduce((a, v) => ({ ...a, [v.name]: v.icon }), {}),
  };

  // Add icon class
  tagIcons = Object.fromEntries(Object.entries(tagIcons).map(([k, v]) => [k, v.replace('<svg', '<svg class="icon"')]));

  // Delete icons from original arrays
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  categories = categories.map(({ icon, ...rest }) => rest);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  loaders = loaders.map(({ icon, ...rest }) => rest);

  // Set project types
  const projectTypes = ['mod', 'modpack'];

  // Write JSON file
  await fs.writeFile(
    GENERATED_PATH + 'tags.json',
    JSON.stringify({ categories, loaders, projectTypes, licenses, donationPlatforms, tagIcons })
  );

  console.log('Generated tags.json');
})();
