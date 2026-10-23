import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';

const html=readFileSync('web/index.html','utf8');
function extract(name){
 const source=html.match(new RegExp('function '+name+'\\([^]*?\\n\\}'));
 assert.ok(source,'Actual browser function '+name+' missing');
 return runInNewContext('('+source[0]+')');
}
const filter=extract('filterStoreEntries');
const validate=extract('validatedStoreRepositories');
const apps=Array.from({length:1317},(_,i)=>({
 slug:'future_'+i,repository:'unknown_repo_'+(i%23),
 name:'Future Provider '+i
}));
apps[1316].name='<img src=x onerror=alert(1)> Unknown Vendor';
assert.equal(filter(apps,'','').length,1317);
assert.equal(filter(apps,'','unknown_repo_7').length,57);
assert.equal(filter(apps,'provider 306','unknown_repo_7').length,1);
assert.equal(filter(apps,'<img','').length,1);
assert.equal(filter(apps,'<img','unknown_repo_5').length,1);
assert.equal(filter(apps,'<img','unknown_repo_7').length,0);
const repos=Array.from({length:23},(_,i)=>({
 slug:'unknown_repo_'+i,name:'Third-party '+String(22-i).padStart(2,'0')
}));
repos[7].name='<svg onload=alert(1)> harmless text';
const sorted=validate(repos);
assert.equal(sorted.length,23);
assert.ok(sorted.some(x=>x.name.includes('<svg')));
for(const bad of [
 [repos[0],repos[0]],
 [{slug:'../etc',name:'unsafe'}],
 [{slug:'valid',name:'\nunsafe'}],
 [{slug:'valid',name:''}],
 [{slug:'valid',name:'x'.repeat(257)}],
 Array.from({length:2049},(_,i)=>({slug:'r'+i,name:'n'}))
])assert.throws(()=>validate(bad));
assert.equal(filter(apps,'','unknown_repo_999').length,0);
console.log('DYNAMIC_STORE_REPOSITORY_FILTER_NODE_PASS: 1317 arbitrary Apps, 23 repos, adversarial metadata');
