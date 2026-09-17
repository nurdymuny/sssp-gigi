// Distance-field reconstruction on verified graph geometry. No event timestamps are inferred.
export function createNetworkView(container,geometry,methods){
  const W=600,H=410,BINS=64,palette=['#7da9ca','#649dc1','#488fb4','#317fa4','#226e90','#175c79'];
  let panels=[],current=null,source=0;
  const point=(g,i)=>[113+g.nodes[i*3]/65535*374,18+g.nodes[i*3+1]/65535*374];
  const band=(g,i)=>g.nodes[i*3+2]===65535?-1:Math.min(BINS-1,Math.floor(g.nodes[i*3+2]/65534*(BINS-1)));
  function configure(scenario,run){
    current=geometry[`${scenario.id}-${run.seed}-${run.source}`];source=run.source;
    if(!current)throw new Error('Missing verified network geometry');
    const g=current,n=g.nodes.length/3;
    const nodePaths=Array.from({length:BINS},()=>new Path2D()),edgePaths=Array.from({length:BINS},()=>new Path2D()),baseEdges=new Path2D();
    const bins=Array.from({length:BINS},()=>0);let unreachable=0;
    const radius=g.layout==='grid'?1.7:g.layout==='geographic'?1.05:2.0;
    for(let i=0;i<n;i++){const b=band(g,i);if(b<0){unreachable++;continue}const [x,y]=point(g,i);nodePaths[b].moveTo(x+radius,y);nodePaths[b].arc(x,y,radius,0,Math.PI*2);bins[b]++;}
    for(let j=0;j<g.edges.length;j+=2){const u=g.edges[j],v=g.edges[j+1],[x,y]=point(g,u),[a,b]=point(g,v);baseEdges.moveTo(x,y);baseEdges.lineTo(a,b);const bu=band(g,u),bv=band(g,v);if(bu>=0&&bv>=0){const p=edgePaths[Math.max(bu,bv)];p.moveTo(x,y);p.lineTo(a,b)}}
    const base=document.createElement('canvas');base.width=W;base.height=H;const ctx=base.getContext('2d');ctx.fillStyle='#fcfcfc';ctx.fillRect(0,0,W,H);ctx.strokeStyle=g.layout==='topological'?'#eceef0':'#dfe4e8';ctx.lineWidth=g.layout==='topological'?.4:.7;ctx.stroke(baseEdges);
    const max=Math.max(...Object.values(run.times));
    container.replaceChildren();panels=[];
    for(const {arm} of scenario.summary){
      if(run.times[arm]===undefined)continue;
      const article=document.createElement('article');article.className='network-panel';article.dataset.arm=arm;
      article.innerHTML=`<div class="network-panel-title"><b>${methods[arm][0]}</b><span>${run.times[arm].toFixed(3)} ms</span></div><canvas width="600" height="410" role="img" aria-label="${methods[arm][0]}: source-distance reconstruction on ${scenario.title}"></canvas><div class="network-panel-status"><span>Ready</span><span class="network-population"></span></div>`;
      container.append(article);const canvas=article.querySelector('canvas');
      panels.push({article,ctx:canvas.getContext('2d'),duration:run.times[arm]/max,last:-2,base,nodePaths,edgePaths,bins,unreachable,n,radius});
    }
    return {view:g,vertices:n,arcs:g.edges.length/2,unreachable};
  }
  function draw(fraction,mode='ready'){
    for(const p of panels){
      const local=Math.min(1,Math.max(0,fraction/p.duration));
      const end=local===0?-1:local===1?BINS-1:Math.floor(local*(BINS-1));
      const tag=`${end}-${mode==='ready'?'ready':local===1?'done':'active'}`;
      if(p.last===tag)continue;p.last=tag;
      const c=p.ctx;c.clearRect(0,0,W,H);c.drawImage(p.base,0,0);
      let populated=0;
      for(let k=0;k<=end;k++){c.strokeStyle=palette[Math.min(palette.length-1,Math.floor(k/BINS*palette.length))];c.lineWidth=current.layout==='topological'?.65:.95;c.globalAlpha=current.layout==='topological'?.3:.75;c.stroke(p.edgePaths[k]);c.globalAlpha=1;c.fillStyle=c.strokeStyle;c.fill(p.nodePaths[k]);populated+=p.bins[k]}
      if(local<1&&fraction>0){c.fillStyle='#cf8d32';c.fill(p.nodePaths[end]);c.strokeStyle='#cf8d32';c.lineWidth=1.3;c.stroke(p.edgePaths[end]);}
      const [sx,sy]=point(current,current.sourceIndex);c.globalAlpha=1;c.fillStyle='#a43649';c.strokeStyle='white';c.lineWidth=2;c.beginPath();c.arc(sx,sy,5,0,Math.PI*2);c.fill();c.stroke();
      c.font='14px Arial';c.fillStyle='#555';c.textAlign='left';c.fillText(`s = ${source}`,Math.min(W-100,sx+9),Math.max(16,sy-9));
      p.article.querySelector('.network-panel-status>span').textContent=mode==='ready'?'Distance field':local===1?'Recorded finish reached':'Illustrative distance wave';
      p.article.querySelector('.network-population').textContent=`${populated.toLocaleString()} / ${p.n.toLocaleString()} shown`;
    }
  }
  return {configure,draw};
}
