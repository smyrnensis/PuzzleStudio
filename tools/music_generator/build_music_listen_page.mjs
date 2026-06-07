import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const outputPath = path.join(root, "music_listen.html");
const templatePath = path.join(root, "music_listen.template.html");

function moduleBody(source) {
  return source
    .split(/\r?\n/)
    .filter((line) => !line.trimStart().startsWith("import "))
    .join("\n")
    .replaceAll("export const ", "const ")
    .replaceAll("export async function ", "async function ")
    .replaceAll("export function ", "function ");
}

function exposeModule(source, exports) {
  return `${moduleBody(source)}\nreturn { ${exports.join(", ")} };`;
}

async function source(name) {
  return readFile(path.join(root, name), "utf8");
}

async function build() {
  const [template, timbreFields, player, music] = await Promise.all([
    readFile(templatePath, "utf8"),
    source("seeded_timbre_fields.mjs"),
    source("seeded_music_player.mjs"),
    source("seeded_music.mjs"),
  ]);

  const bundle = `(() => {
  const musicPlayer = (() => {
${exposeModule(player, ["createPlayer"])}
  })();
  const musicGenerator = (() => {
${moduleBody(timbreFields)}
${exposeModule(music, ["generateSong", "randomPreset"])}
  })();
  window.PuzzleMusicListenTools = {
    createPlayer: musicPlayer.createPlayer,
    generateSong: musicGenerator.generateSong,
    randomPreset: musicGenerator.randomPreset,
  };
  window.dispatchEvent(new CustomEvent("PuzzleMusicListenToolsReady"));
})();`;

  return template.replace(
    "/* PUZZLESTUDIO_MUSIC_TOOLS_BUNDLE */",
    bundle.replaceAll("</script", "<\\/script"),
  );
}

const html = await build();
const args = new Set(process.argv.slice(2));

if (args.has("--check")) {
  const current = await readFile(outputPath, "utf8");
  if (current !== html) {
    throw new Error("music_listen.html is out of date. Run node tools/music_generator/build_music_listen_page.mjs.");
  }
} else {
  await writeFile(outputPath, html);
  console.log(`wrote ${path.relative(process.cwd(), outputPath)}`);
}
