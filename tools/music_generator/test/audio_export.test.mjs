import assert from "node:assert/strict";
import { exportMusicLoop, exportSoundEffect } from "../audio_export.mjs";
import { generateSong } from "../seeded_music.mjs";
import { generateSoundEffect } from "../seeded_sfx.mjs";

const randomTargetSfx = generateSoundEffect("123456", { type: "random" });
const randomExport = exportSoundEffect(randomTargetSfx, "random");
assert.equal(randomExport.kind, "sfx");
assert.equal(randomExport.seed, "123456");
assert.equal(randomExport.type, randomTargetSfx.type, "Random exports the concrete type resolved from the seed");
assert.equal(randomExport.ps, `sfx ${randomTargetSfx.type} 123456`);

const wildExport = exportSoundEffect(generateSoundEffect("654321", { type: "wild" }), "wild");
assert.equal(wildExport.type, "wild");
assert.equal(wildExport.ps, "sfx wild 654321");

const songExport = exportMusicLoop(generateSong("music-seed", { tone: 0.62, bpm: 106, volume: 0.7 }));
assert.equal(songExport.kind, "music");
assert.equal(songExport.seed, "music-seed");
assert.equal(songExport.tone, 0.62);
assert.equal(songExport.height, 0.5);
assert.equal(songExport.brightness, 0.62);
assert.equal(songExport.bpm, 106);
assert.equal(songExport.volume, 0.7);
assert.equal(songExport.bars, 8);
assert.equal(songExport.ps, "music music-seed height=0.5 bpm=106 bars=8 volume=0.7");

const musicExport = exportMusicLoop(generateSong("music-seed", { height: 0.8, bpm: 110, bars: 16, volume: 0.5 }));
assert.equal(musicExport.height, 0.8);
assert.equal(musicExport.brightness, 0.5);
assert.equal(musicExport.presence, 0.5);
assert.equal(musicExport.attack, 0.5);
assert.equal(musicExport.bars, 16);
assert.equal(musicExport.ps, "music music-seed height=0.8 bpm=110 bars=16 volume=0.5");

console.log("audio_export tests passed");
