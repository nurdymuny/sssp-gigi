import study from './data.mjs';
import geometry from './geometry.mjs';
import {createNetworkView} from './network-view.mjs';
import {makeGraph,heapTrace,bucketTrace} from './engine.mjs';
const $=id=>document.getElementById(id);
const motion=window.matchMedia('(prefers-reduced-motion: reduce)');
const number=new Intl.NumberFormat('en-US');
const methods={std_binary:['Std heap','Dijkstra','heap'],original_mean:['List','Append-only','list'],mean_map:['Map','Deduplicated','map'],mean_ring:['Ring','Cyclic buckets','ring'],max_maxdegree:['Max / degree','Width heuristic','max'],selected_grid:['Calibrated','Tuning excluded','selected']};
const networkView=createNetworkView($('network-panels'),geometry,methods),replayDuration=8000;
const overview=[['sparse','original_mean'],['million','mean_ring'],['ny','mean_ring'],['bay','mean_ring'],['col','mean_ring'],['dense','original_mean'],['dense','mean_map']];
$('overview-results').innerHTML=overview.map(([id,arm])=>{const c=study.scenarios.find(x=>x.id===id),v=c.summary.find(x=>x.arm===arm);return `<tr><td>${c.family==='sparse_fast'?'Sparse (scale)':c.family==='sparse'?'Sparse (core)':c.family==='dense'?'Dense (core)':'DIMACS '+c.family}</td><td>${number.format(c.n)}</td><td>${methods[arm][0]}</td><td>${v.ratio.toFixed(id==='dense'?3:2)}</td></tr>`}).join('');
const stories={
  sparse:['Sparse core / n = 10,000','List reaches a 1.97× aggregate speed ratio on these 10,000-vertex sparse graphs. Ring has larger paired ratios at the three smaller sparse sizes. Small differences do not establish a resolved ordering.'],
  million:['Sparse scale / n = 1,000,000','At one million vertices, Map and Ring reach aggregate speed ratios of 1.81× and 1.82×. This panel uses a sparse generator with the same stated distribution but a different seeded stream from the core panel.'],
  ny:['DIMACS NY / competing width rules','On New York, Ring reaches 1.64× and the maximum-weight/maximum-degree rule reaches 1.74×. The calibrated width reaches 1.73× before charging for its search. These comparisons also involve different bucket containers.'],
  bay:['DIMACS BAY / competing width rules','On the Bay Area graph, Ring reaches 1.51× and the maximum-weight/maximum-degree rule reaches 1.57×. Calibration reaches 1.58× before its preparation cost. Those modest differences do not establish a universal winner.'],
  col:['DIMACS COL / competing width rules','On Colorado, Ring reaches 1.57×, the maximum-weight/maximum-degree rule 1.55×, and calibration 1.57× before tuning cost. Both search-free rules remain competitive with calibration.'],
  dense:['Dense core / residual bucket overhead','On dense graphs with 5,000 vertices, List reaches only 0.063× the baseline’s speed. Duplicate suppression improves Map to 0.564×, but that is still about 1.77 times as long as the heap.'],
  grid:['Grid core / n = 10,000','On the 10,000-vertex grid, List reaches 1.56×, Map 1.38×, and Ring 1.47×. Across the tested grid sizes and these mean-width implementations, ratios range from 0.95× to 1.56×.']
};
let scenario=study.scenarios[0],run=null,raceMode='ready',raceFrame=0,raceElapsed=0,raceLast=0;
const options=(el,values,format=x=>String(x))=>{el.replaceChildren(...values.map(v=>{const o=document.createElement('option');o.value=v;o.textContent=format(v);return o}))};
options($('scenario'),study.scenarios.map(s=>s.id),id=>study.scenarios.find(s=>s.id===id).title);
function fillSeeds(){const seeds=[...new Set(scenario.runs.map(x=>x.seed))];options($('seed'),seeds);$('seed-wrap').hidden=scenario.panel==='roads';fillSources()}
function fillSources(){const seed=Number($('seed').value);options($('source'),[...new Set(scenario.runs.filter(x=>x.seed===seed).map(x=>x.source))],x=>number.format(x));fillRounds()}
function fillRounds(){options($('round'),scenario.runs.filter(x=>x.seed===Number($('seed').value)&&x.source===Number($('source').value)).map(x=>x.round),x=>String(x+1));selectRun()}
function selectRun(){
  cancelAnimationFrame(raceFrame);raceMode='ready';raceElapsed=0;
  run=scenario.runs.find(x=>x.seed===Number($('seed').value)&&x.source===Number($('source').value)&&x.round===Number($('round').value));
  const network=networkView.configure(scenario,run);
  $('network-meta').textContent=`${network.view.layout==='geographic'?'Geographic coordinates':network.view.layout==='grid'?'100 × 100 lattice':'Schematic topology; positions have no geographic meaning'} · ${number.format(network.vertices)} displayed vertices / ${number.format(network.arcs)} arcs · ${network.view.sampling}.${network.unreachable?` ${number.format(network.unreachable)} displayed vertices are unreachable from this source.`:''}`;
  $('race').innerHTML=scenario.summary.filter(x=>run.times[x.arm]!==undefined).map(x=>{const [name,note,cls]=methods[x.arm];return `<div class="race-row" data-arm="${x.arm}"><div class="race-label">${name}<small>${note}</small></div><div class="race-track"><div class="race-bar ${cls}"></div></div><div class="race-value">${run.times[x.arm].toFixed(3)} ms<small>recorded</small></div></div>`}).join('');
  $('run-meta').textContent=`${number.format(scenario.n)} vertices · source ${number.format(run.source)} · round ${run.round+1}${scenario.panel==='roads'?'':` · seed ${run.seed}`}`;
  $('scenario-headline').textContent=stories[scenario.id][0];$('scenario-story').textContent=stories[scenario.id][1];
  $('aggregate').innerHTML=scenario.summary.filter(x=>x.arm!=='std_binary').map(x=>`<div><span>${methods[x.arm][0]}</span><b>${x.ratio.toFixed(2)}×</b></div>`).join('')+'<span class="aggregate-caption">Aggregate paired speed ratios. Above 1× is faster than the study’s standard heap. These are not ratios for the single round above.</span>';
  $('race-play').textContent='Replay timings';$('race-status').textContent='Recorded solve durations; milliseconds.';drawRace(1);
}
function drawRace(fraction){
  networkView.draw(fraction,raceMode);$('race-scrub').value=Math.round(fraction*1000);$('race-position').textContent=`${Math.round(fraction*100)}%`;
  const longest=Math.max(...Object.values(run.times));
  for(const el of $('race').children){const duration=run.times[el.dataset.arm],ratio=duration/longest;const done=fraction>=ratio;el.querySelector('.race-bar').style.width=`${Math.min(fraction,ratio)*100}%`;el.classList.toggle('done',done);el.querySelector('.race-value small').textContent=raceMode==='ready'?'recorded':done?'finished':'running';}
}
function tickRace(now){if(raceMode!=='playing')return;raceElapsed+=now-raceLast;raceLast=now;const f=Math.min(1,raceElapsed/replayDuration);drawRace(f);if(f<1)raceFrame=requestAnimationFrame(tickRace);else{raceMode='complete';$('race-play').textContent='Replay timings';$('race-status').textContent='Replay complete. Exact saved times are shown.'}}
$('race-play').addEventListener('click',()=>{
  if(motion.matches){raceMode='complete';drawRace(1);$('race-status').textContent='Reduced motion: all saved finish times shown.';return}
  if(raceMode==='playing'){raceMode='paused';cancelAnimationFrame(raceFrame);$('race-play').textContent='Resume timings';$('race-status').textContent='Paused.';return}
  if(raceMode!=='paused')raceElapsed=0;
  raceMode='playing';raceLast=performance.now();$('race-play').textContent='Pause';$('race-status').textContent='Recorded finish times; illustrative distance waves. Eight-second display window.';drawRace(raceElapsed/replayDuration);raceFrame=requestAnimationFrame(tickRace);
});
$('race-scrub').addEventListener('input',()=>{cancelAnimationFrame(raceFrame);raceElapsed=Number($('race-scrub').value)/1000*replayDuration;raceMode='paused';drawRace(raceElapsed/replayDuration);$('race-play').textContent='Resume timings';$('race-status').textContent='Paused at the selected replay position; color shows a reconstructed distance wave.'});
$('race-reset').addEventListener('click',selectRun);
$('scenario').addEventListener('change',()=>{scenario=study.scenarios.find(s=>s.id===$('scenario').value);fillSeeds()});
$('seed').addEventListener('change',fillSources);$('source').addEventListener('change',fillRounds);$('round').addEventListener('change',selectRun);fillSeeds();

let graph,heap,bucket,step=0,toyTimer=0,toyPlaying=false;
function stopToy(){clearTimeout(toyTimer);toyPlaying=false;$('toy-play').textContent='Replay traces'}
function resetToy(){
  stopToy();step=0;graph=makeGraph($('toy-network').value);const mean=graph.edges.reduce((s,e)=>s+e.w,0)/graph.edges.length;
  const factor={mean:1,small:.5,large:2}[$('toy-width').value];heap=heapTrace(graph);bucket=bucketTrace(graph,mean*factor);
  const equal=heap.at(-1).d.every((d,i)=>d===bucket.at(-1).d[i]);
  if(!equal){$('toy-verdict').textContent='Distance validation failed. Animation unavailable.';$('toy-play').disabled=true;$('toy-step').disabled=true;return}
  $('toy-play').disabled=false;$('toy-step').disabled=false;$('toy-verdict').textContent='';renderToy();
}
function renderGraph(id,state){
  const svg=$(id),arrow=`arrow-${id}`;const active=new Set(state.active),edges=new Set(state.edges);
  const links=graph.edges.map(e=>{const a=graph.nodes[e.u],b=graph.nodes[e.v],dx=b.x-a.x,dy=b.y-a.y,len=Math.hypot(dx,dy),ux=dx/len,uy=dy/len;const ax=a.x+ux*20,ay=a.y+uy*20,bx=b.x-ux*24,by=b.y-uy*24;const show=$('show-costs').checked&&(graph.kind==='sparse'||edges.has(e.id));return `<g><path d="M${ax} ${ay}L${bx} ${by}" class="edge ${edges.has(e.id)?'active':''}" marker-end="url(#${arrow})"/>${show?`<text class="cost" text-anchor="middle" x="${(ax+bx)/2-uy*8}" y="${(ay+by)/2+ux*8}">${e.w}</text>`:''}</g>`}).join('');
  const nodes=graph.nodes.map(n=>`<g class="vertex ${Number.isFinite(state.d[n.id])?'known':''} ${active.has(n.id)?'active':''}"><circle cx="${n.x}" cy="${n.y}" r="18"/><text x="${n.x}" y="${n.y}">${String.fromCharCode(65+n.id)}</text><text class="distance" x="${n.x}" y="${n.y+32}">${Number.isFinite(state.d[n.id])?state.d[n.id]:'∞'}</text></g>`).join('');
  svg.innerHTML=`<title>${id==='heap-graph'?'Dijkstra':'Delta stepping'}: ${state.message}</title><defs><marker id="${arrow}" markerWidth="7" markerHeight="7" refX="6" refY="3" orient="auto" markerUnits="userSpaceOnUse"><path d="M0,0 L6,3 L0,6" fill="none" stroke="#888888" stroke-width="1.2"/></marker></defs>${links}${nodes}`;
}
function renderToy(){
  const a=heap[Math.min(step,heap.length-1)],b=bucket[Math.min(step,bucket.length-1)];
  for(const [prefix,state] of [['heap',a],['bucket',b]]){
    renderGraph(`${prefix}-graph`,state);$(`${prefix}-checks`).textContent=state.checks;$(`${prefix}-reached`).textContent=state.d.filter(Number.isFinite).length;$(`${prefix}-message`).textContent=state.message;
    $(`${prefix}-queue`).innerHTML=`<span class="queue-label">${prefix==='heap'?'Waiting:':'Group:'}</span>`+(state.queue.length?state.queue.slice(0,10).map(x=>`<span>${x}</span>`).join(''):'<span>none</span>');
  }
  if(a.done&&b.done){stopToy();$('toy-play').textContent='Replay traces';$('toy-step').disabled=true;$('toy-verdict').textContent='Distance agreement: 12 / 12 vertices.'}
}
function advanceToy(){if(step<Math.max(heap.length,bucket.length)-1){step++;renderToy()}}
function scheduleToy(){toyTimer=setTimeout(()=>{if(!toyPlaying)return;advanceToy();if(toyPlaying)scheduleToy()},800)}
$('toy-play').addEventListener('click',()=>{if(toyPlaying){stopToy();$('toy-play').textContent='Resume traces';return}if(step>=Math.max(heap.length,bucket.length)-1){step=0;$('toy-step').disabled=false;$('toy-verdict').textContent='';renderToy()}if(motion.matches){step=Math.max(heap.length,bucket.length)-1;renderToy();return}toyPlaying=true;$('toy-play').textContent='Pause';scheduleToy()});
$('toy-step').addEventListener('click',()=>{stopToy();advanceToy()});$('toy-reset').addEventListener('click',resetToy);$('toy-network').addEventListener('change',resetToy);$('toy-width').addEventListener('change',resetToy);$('show-costs').addEventListener('change',renderToy);resetToy();

const chart=$('density-chart'),x=i=>58+i*75,y=r=>232-(r-.5)*143;
chart.innerHTML=`<title>Mean-width Map speed ratio by expected outgoing degree on 5,000-vertex graphs</title><line class="threshold" x1="48" y1="${y(1)}" x2="606" y2="${y(1)}"/><text x="52" y="${y(1)-10}">1× = same time as heap</text><path class="trend" d="${study.density.map((d,i)=>`${i?'L':'M'}${x(i)},${y(d.ratio)}`).join(' ')}"/>${study.density.map((d,i)=>`<circle class="point" cx="${x(i)}" cy="${y(d.ratio)}" r="4"/><text text-anchor="middle" x="${x(i)}" y="264">${d.degree}</text>`).join('')}<circle id="density-selected" class="selected-point" r="8" cx="${x(1)}" cy="${y(study.density[1].ratio)}"/><text x="16" y="${y(1.5)+5}">1.5×</text><text x="16" y="${y(.5)+5}">0.5×</text>`;
function updateDensity(){const i=Number($('degree').value),d=study.density[i];$('degree-label').textContent=d.degree;$('degree').setAttribute('aria-valuetext',`${d.degree} expected outgoing edges per vertex`);$('density-number').textContent=`${d.ratio.toFixed(2)}×`;$('density-selected').setAttribute('cx',x(i));$('density-selected').setAttribute('cy',y(d.ratio));const neutral=Math.abs(d.ratio-1)<.03;$('density-title').textContent=neutral?'Near parity with the baseline':d.ratio>1?'Mean-width Map above baseline':'Mean-width Map below baseline';$('density-story').textContent=neutral?'At this degree, the aggregate solve times are nearly equal. A small median difference is not a resolved win.':d.ratio>1?`About ${Math.round((1-1/d.ratio)*100)}% less solve time in the aggregate comparison. This comparison is conditional on the graph family and degree.`:`About ${(1/d.ratio).toFixed(2)} times the heap’s solve time. The ranking depends on the graph distribution.`;$('density-interval').textContent=`Descriptive three-seed interval: ${d.lo.toFixed(2)}–${d.hi.toFixed(2)}×${d.lo<=1&&d.hi>=1?'; it spans equal performance.':'.'} These are observations, not a universal cutoff.`}
$('degree').addEventListener('input',updateDensity);updateDensity();
const before=study.mechanisms.find(x=>x.arm==='list_both'),after=study.mechanisms.find(x=>x.arm==='mean_map');$('heavy-before').textContent=number.format(Math.round(Number(before.heavy)));$('heavy-after').textContent=number.format(Math.round(Number(after.heavy)));$('heavy-after-bar').style.width=`${Number(after.heavy)/Number(before.heavy)*100}%`;
document.addEventListener('visibilitychange',()=>{if(document.hidden){if(raceMode==='playing'){raceMode='paused';cancelAnimationFrame(raceFrame);$('race-play').textContent='Resume timings';$('race-status').textContent='Paused while this page is hidden.'}if(toyPlaying)stopToy()}});
motion.addEventListener('change',()=>{if(motion.matches){if(raceMode==='playing'){cancelAnimationFrame(raceFrame);raceMode='complete';drawRace(1);$('race-play').textContent='Replay timings';$('race-status').textContent='Reduced motion: saved finish times shown.'}stopToy()}});
