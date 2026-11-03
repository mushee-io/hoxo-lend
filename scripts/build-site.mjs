import fs from "node:fs";
import path from "node:path";
import { build } from "esbuild";

const root=process.cwd();
const output=path.join(root,"dist");
fs.rmSync(output,{force:true,recursive:true});
fs.mkdirSync(output,{recursive:true});
for(const name of ["index.html","app","docs","config","src"]){
  const src=path.join(root,name);
  const dest=path.join(output,name);
  fs.cpSync(src,dest,{recursive:true});
}
await build({
  entryPoints:["src/tx-live.js"],
  bundle:true,
  platform:"browser",
  target:"es2022",
  format:"iife",
  outfile:"dist/src/tx-live.bundle.js",
  sourcemap:false,
  minify:true,
  logLevel:"info",
  define:{"process.env.NODE_ENV":'"production"'}
});
console.log("Hoxo Lend Stellar Testnet browser runtime bundled at dist/src/tx-live.bundle.js");
