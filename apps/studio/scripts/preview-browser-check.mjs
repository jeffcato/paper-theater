process.env.PT_STUDIO_PLAYER_CHECK='1';
process.env.PT_STUDIO_FIXTURE='fixtures/generation/two-scene-readiness-v2';
if(process.argv.includes('--preview-only'))process.env.PT_STUDIO_PREVIEW_ONLY='1';
await import('./browser-check.mjs');
