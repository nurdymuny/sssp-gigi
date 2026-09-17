from pathlib import Path
import re
base=Path(__file__).resolve().parent.parent
dist=base/'shortest-paths-site/dist'
html=(dist/'index.html').read_text(encoding='utf-8')
css=(dist/'styles.css').read_text(encoding='utf-8')
css=re.sub(r"@import url\([^;]+;\n",'',css)
engine=(dist/'engine.mjs').read_text(encoding='utf-8').replace('export function','function')
data=(dist/'data.mjs').read_text(encoding='utf-8').replace('export default ','const study = ',1)
geometry=(dist/'geometry.mjs').read_text(encoding='utf-8').replace('export default ','const geometry = ',1)
network=(dist/'network-view.mjs').read_text(encoding='utf-8').replace('export function','function')
app=(dist/'app.mjs').read_text(encoding='utf-8')
app='\n'.join(line for line in app.splitlines() if not line.startswith('import '))
script=engine+'\n'+data+'\n'+geometry+'\n'+network+'\n'+app
assert '</script' not in script.lower()
html=html.replace('<link rel="stylesheet" href="styles.css">','<style>'+css+'</style>')
html=html.replace('<script type="module" src="app.mjs"></script>','<script>'+script+'</script>')
html=html.replace('href="evidence/','href="shortest-paths-site/dist/evidence/')
target=base/'shortest-paths-offline.html'
target.write_text(html,encoding='utf-8')
(base/'tmp/offline_site_check.js').write_text(script,encoding='utf-8')
print('Self-contained interactive page saved; no server needed for animations.')
