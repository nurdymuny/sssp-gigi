// Teaching algorithms. These traces are computed here, never presented as benchmark traces.
export function makeGraph(kind='sparse') {
  const nodes=Array.from({length:12},(_,i)=>({id:i,x:52+(i%4)*144,y:48+Math.floor(i/4)*100}));
  const sparse=[[0,1,4],[0,4,2],[0,5,9],[1,2,3],[1,5,2],[2,3,5],[2,6,1],[3,7,3],[4,5,1],[4,8,6],[5,6,4],[5,9,2],[6,7,2],[6,10,3],[7,11,2],[8,9,2],[9,10,2],[9,6,1],[10,11,1]];
  const tuples=kind==='dense'?nodes.flatMap((u)=>nodes.filter(v=>v.id>u.id).map(v=>[u.id,v.id,1+((u.id*13+v.id*7)%19)])):sparse;
  return {nodes,edges:tuples.map(([u,v,w],id)=>({u,v,w,id})),kind};
}
const initial=(n)=>Array.from({length:n},(_,i)=>i===0?0:Infinity);
const cloneState=(d,active,edges,checks,message,queue,done=false)=>({d:[...d],active:[...active],edges:[...edges],checks,message,queue:[...queue],done});
const label=(v)=>String.fromCharCode(65+v);
const fmt=(v)=>Number.isInteger(v)?String(v):v.toFixed(1);
export function heapTrace(g) {
  const d=initial(g.nodes.length), settled=new Set(), q=[{v:0,d:0}];let checks=0;
  const out=[cloneState(d,[],[],0,'Initialize D[A] = 0; all other tentative labels are infinite.',['A · 0'])];
  while(q.length) {
    q.sort((a,b)=>a.d-b.d||a.v-b.v);const cur=q.shift();if(settled.has(cur.v)||cur.d!==d[cur.v])continue;
    settled.add(cur.v);const improved=[];
    for(const e of g.edges.filter(e=>e.u===cur.v)) {
      checks++;const candidate=d[cur.v]+e.w;
      if(candidate<d[e.v]){d[e.v]=candidate;q.push({v:e.v,d:candidate});improved.push(e.id)}
    }
    const waiting=[...new Map(q.filter(x=>!settled.has(x.v)&&x.d===d[x.v]).map(x=>[x.v,x])).values()].sort((a,b)=>a.d-b.d||a.v-b.v);
    out.push(cloneState(d,[cur.v],improved,checks,`Extract ${label(cur.v)} at minimum label ${fmt(cur.d)}; settle the vertex and relax its outgoing edges.`,waiting.map(x=>`${label(x.v)} · ${fmt(x.d)}`)));
  }
  out.push(cloneState(d,[],[],checks,'Heap exhausted. All reachable vertices have their shortest-distance labels.',[],true));return out;
}
export function bucketTrace(g,width) {
  if(!(width>0))throw new Error('Group width must be positive');
  const d=initial(g.nodes.length), last=Array(g.nodes.length).fill(Infinity), buckets=new Map([[0,[0]]]);let checks=0, guard=0;
  const out=[cloneState(d,[],[],0,`Initialize source A in bucket 0 with width Δ = ${fmt(width)}.`,[`0 ≤ D[v] < ${fmt(width)}`])];
  function relax(e,improved) {
    checks++;const candidate=d[e.u]+e.w;
    if(candidate<d[e.v]){d[e.v]=candidate;const i=Math.floor(candidate/width);if(!buckets.has(i))buckets.set(i,[]);buckets.get(i).push(e.v);improved.push(e.id)}
  }
  while(buckets.size) {
    if(++guard>10000)throw new Error('Teaching trace exceeded its bound');
    const i=Math.min(...buckets.keys()), touched=new Set();
    const range=`${fmt(i*width)} ≤ D[v] < ${fmt((i+1)*width)}`;
    while(buckets.has(i)) {
      const entries=buckets.get(i);buckets.delete(i);const active=[],improved=[];
      for(const u of entries) {
        if(Math.floor(d[u]/width)!==i||last[u]===d[u])continue;
        last[u]=d[u];touched.add(u);active.push(u);
        for(const e of g.edges.filter(e=>e.u===u&&e.w<=width))relax(e,improved);
      }
      if(active.length)out.push(cloneState(d,active,improved,checks,`Bucket ${i}: relax light edges from ${active.map(label).join(', ')}. Repeat until the bucket reaches light closure.`,[range]));
    }
    const improved=[];
    for(const u of touched)for(const e of g.edges.filter(e=>e.u===u&&e.w>width))relax(e,improved);
    if(touched.size)out.push(cloneState(d,[...touched],improved,checks,`Close bucket ${i}; process each touched vertex’s heavy edges once using its final label for this phase.`,[range,'Unique heavy-edge processing']));
  }
  out.push(cloneState(d,[],[],checks,'All buckets exhausted. All reachable vertices have their shortest-distance labels.',[],true));return out;
}
export function bellmanFord(g) {
  const d=initial(g.nodes.length);
  for(let i=1;i<g.nodes.length;i++){let changed=false;for(const e of g.edges){if(d[e.u]+e.w<d[e.v]){d[e.v]=d[e.u]+e.w;changed=true}}if(!changed)break}
  return d;
}
