import assert from 'node:assert/strict';
import {readFileSync,existsSync} from 'node:fs';
import {makeGraph,heapTrace,bucketTrace,bellmanFord} from './dist/engine.mjs';
import data from './dist/data.mjs';
import geometry from './dist/geometry.mjs';
// Independent distance recurrence checks the teaching algorithms, including cycles,
// zero weights, stale entries and unreachable places. No timing claims are made.
let graphs=0,comparisons=0;
function check(g){
  const expected=bellmanFord(g);assert.deepEqual(heapTrace(g).at(-1).d,expected);comparisons++;
  const mean=g.edges.reduce((s,e)=>s+e.w,0)/Math.max(1,g.edges.length);
  for(const width of [.25,1,mean||1,(mean||1)*2,50]){const trace=bucketTrace(g,width);assert.deepEqual(trace.at(-1).d,expected);assert(trace.at(-1).done);comparisons++}
  graphs++;
}
check(makeGraph('sparse'));check(makeGraph('dense'));
let seed=5842;
function rng(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/2**32}
for(let k=0;k<150;k++){
  const nodes=Array.from({length:9},(_,id)=>({id})),edges=[];
  for(let u=0;u<9;u++)for(let v=0;v<9;v++)if(u!==v&&rng()<.2)edges.push({u,v,w:Math.floor(rng()*12),id:edges.length});
  check({nodes,edges});
}
const html=readFileSync(new URL('./dist/index.html',import.meta.url),'utf8');
for(const [,path] of html.matchAll(/(?:href|src)="([^"#][^"]*)"/g))if(!/^(https?:|mailto:)/.test(path))assert(existsSync(new URL('./dist/'+path,import.meta.url)),path);
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(x=>x[1]);assert.equal(new Set(ids).size,ids.length);
assert(!/\b(first|novel|patented)\b/i.test(html));
let rounds=0;
for(const s of data.scenarios){
  assert(s.runs.length>0);
  for(const r of s.runs){assert(r.times.std_binary>0);assert(Object.values(r.times).every(t=>Number.isFinite(t)&&t>0));assert(s.summary.every(x=>r.times[x.arm]!==undefined));rounds++}
}
assert.equal(rounds,396);
assert.equal(Object.keys(geometry).length,42);
for(const s of data.scenarios)for(const r of s.runs){
  const g=geometry[`${s.id}-${r.seed}-${r.source}`];assert(g,'Missing source geometry');assert.equal(g.fullN,s.n);
}
for(const g of Object.values(geometry)){
  assert.equal(g.nodes.length%3,0);assert.equal(g.edges.length%2,0);
  assert(g.nodes.every(x=>Number.isInteger(x)&&x>=0&&x<=65535));
  assert(g.edges.every(x=>Number.isInteger(x)&&x>=0&&x<g.nodes.length/3));
  assert.equal(g.nodes[g.sourceIndex*3+2],0,'Source distance must be zero');
  if(g.layout==='grid'){assert.equal(g.nodes.length/3,10000);assert.equal(g.edges.length/2,19800)}
}
assert.equal(data.density.length,8);
assert(Math.abs(data.scenarios.find(s=>s.id==='sparse').summary.find(s=>s.arm==='original_mean').ratio-1.97)<.01);
console.log(JSON.stringify({teaching_graphs:graphs,independent_distance_comparisons:comparisons,recorded_rounds:rounds,source_geometries:Object.keys(geometry).length,local_links:'valid',duplicate_ids:0},null,2));
