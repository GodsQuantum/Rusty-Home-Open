// Genuine web/index.html, isolated local server, synthetic fixture data.
// Optional Playwright install under ignored runtime/brandshot.
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(resolve('runtime/brandshot/node_modules/playwright'));
const html = readFileSync('web/index.html');
const readiness = { production_ready:false, deployment_authorized:false,
 release_gates_total:15,release_gates_blocked:15 };
const apps = {read_only:true,degraded:false,attestation:{status:'synthetic'},
 sources:[{host:'lab-alpha',kind:'synthetic_fixture',status:'ok',live:false,count:3},
          {host:'lab-beta',kind:'synthetic_fixture',status:'ok',live:false,count:1}],
 apps:[
 {host:'lab-alpha',backend:'supervisor',display_name:'Demo ESPHome',state:'running',app_type:'esphome',classification:'recognized',version:'simulation'},
 {host:'lab-alpha',backend:'supervisor',display_name:'Demo Music Assistant',state:'running',app_type:'music_assistant',classification:'recognized',version:'simulation'},
 {host:'lab-alpha',backend:'supervisor',display_name:'Demo Matter',state:'stopped',app_type:'matter',classification:'recognized',version:'simulation'},
 {host:'lab-beta',backend:'supervisor',display_name:'Demo File Editor',state:'running',app_type:'configurator',classification:'recognized',version:'simulation'}]
};
const server=createServer((req,res)=>{
 let body,ctype='application/json',status=200;
 if(req.url==='/'){body=html;ctype='text/html; charset=utf-8';}
 else if(req.url==='/api/readiness'){body=JSON.stringify(readiness);}
 else if(req.url==='/api/apps'){body=JSON.stringify(apps);}
 else if(req.url==='/api/python-core'){status=404;body='{}';}
 else {status=404;body='{}';}
 res.writeHead(status,{'Content-Type':ctype,'Cache-Control':'no-store'});res.end(body);
});
try {
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const port=server.address().port;
 const browser=await chromium.launch({headless:true,...(process.env.RUSTY_HOME_CHROME_CHANNEL ? {channel:process.env.RUSTY_HOME_CHROME_CHANNEL}:{}),args:['--no-sandbox']});
 try {
  const page=await browser.newPage({viewport:{width:1365,height:860},deviceScaleFactor:1});
  await page.goto('http://127.0.0.1:'+port+'/',{waitUntil:'load'});
  await page.waitForFunction(()=>document.querySelector('#status')?.textContent==='Inventaire disponible',{timeout:10000});
  if(!(await page.locator('#readiness').innerText()).includes('en développement'))throw Error('Synthetic readiness not visible');
  if((await page.locator('.card').count())!==4)throw Error('Unexpected synthetic app count');
  await page.screenshot({path:'branding/lab-dashboard.png',fullPage:true,animations:'disabled'});
  const mobile = await browser.newPage({viewport:{width:390,height:844},deviceScaleFactor:1});
  await mobile.goto('http://127.0.0.1:'+port+'/',{waitUntil:'load'});
  await mobile.waitForFunction(()=>document.querySelector('#status')?.textContent==='Inventaire disponible');
  if(!(await mobile.locator('#readiness').innerText()).includes('en développement'))throw Error('Mobile release gate state not visible');
  if((await mobile.locator('.card').count())!==4)throw Error('Mobile synthetic app count incorrect');
  await mobile.screenshot({path:'branding/lab-dashboard-mobile.png',fullPage:true,animations:'disabled'});
  // The real UI must never confuse a native Supervisor admin session with
  // permission to perform lifecycle writes. Only synthetic session responses.
  let sessionFetches=0;
  await page.route('**/api/session',async route=>{
   sessionFetches++;
   await route.fulfill({status:200,contentType:'application/json',
    body:JSON.stringify({authenticated:true,administrator:true,
     authority:'supervisor_ingress',actions_authorized:false,read_only:true,
     token:'SHOULD_NOT_RENDER',user_id:'SHOULD_NOT_RENDER'})});
  });
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#session-status')?.textContent?.includes('administrateur vérifiée'));
  if(sessionFetches!==1)throw Error('Admin session not read from native API');
  if((await page.locator('body').innerText()).includes('SHOULD_NOT_RENDER'))
   throw Error('Session API leaked identity into dashboard');
  await page.unroute('**/api/session');
  await page.route('**/api/session',route=>route.fulfill({status:200,
   contentType:'application/json',body:JSON.stringify({
    authenticated:true,administrator:false,authority:'supervisor_ingress',
    actions_authorized:false,read_only:true
   })}));
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#session-status')?.textContent?.includes('droits administrateur absents'));
  console.log('NATIVE_SUPERVISOR_SESSION_UI_PASS: verified admin/non-admin shown without write permit or identity leaks');
  // Browser-level product test: many arbitrary third-party Apps, not 4 recognized names.
  const catalog = Array.from({length:83},(_,i)=>({
   host:'dynamic-lab',backend:i%2===0?'docker':'supervisor',
   display_name:'Future Provider '+String(i).padStart(4,'0'),
   app_type:i%2===0?'generic_container':'supervisor_app',
   classification:i%2===0?'docker':'supervisor',
   state:'running',version:'synthetic'
  }));
  catalog[82].display_name='<img src=x onerror=alert(1)> Future Provider 0082';
  let requests=0;
  await page.route('**/api/apps', async route=>{
   requests++;
   await route.fulfill({status:200,contentType:'application/json',
    body:JSON.stringify({...apps,apps:catalog})});
  });
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#count')?.textContent==='83');
  if((await page.locator('.card').count())!==24)throw Error('Not paginated to 24 visible Apps');
  if(!(await page.locator('#page-status').innerText()).includes('1 / 4'))throw Error('Wrong page count for unknown apps');
  await page.locator('#next-page').click();
  if(!(await page.locator('#page-status').innerText()).includes('2 / 4'))throw Error('Next page incorrect');
  await page.locator('#filter').selectOption('docker');
  if((await page.locator('#count').innerText())!=='42 / 83')throw Error('Unexpected source filter total');
  await page.locator('#search').fill('Future Provider 0050');
  if((await page.locator('.card').count())!==1)throw Error('Generic name search incorrect');
  if(requests!==1)throw Error('UI filtering must not re-fetch private App inventory');
  await page.locator('#search').fill('<img');
  if((await page.locator('article.card img').count())!==0)throw Error('Untrusted App name injected HTML');
  if((await page.locator('.card').count())!==1)throw Error('Unsafe App name hidden');
  // A new unknown live App can expose official redacted detail, never credentials.
  // No production user, transport, token or App is used in this browser lab.
  await page.unroute('**/api/apps');
  await page.locator('#filter').selectOption('all');
  await page.locator('#search').fill('');
  const liveSlug='future_vendor_92021';
  await page.route('**/api/apps', async route=>route.fulfill({
   status:200,contentType:'application/json',
   body:JSON.stringify({...apps,sources:[{host:'test-supervisor',kind:'supervisor-live',live:true,status:'ok',count:1}],apps:[{
    host:'test-supervisor',backend:'supervisor',live:true,slug:liveSlug,
    display_name:'Future Third Party',state:'started',
    app_type:'supervisor_app',version:'1.0',classification:'supervisor'
   }]})
  }));
  let detailCalls=0;
  await page.route('**/api/apps/'+liveSlug+'/detail', async route=>{
   detailCalls++;
   await route.fulfill({status:200,contentType:'application/json',
    body:JSON.stringify({read_only:true,app:{
      slug:liveSlug,name:'Future Third Party',version:'1.0',
      repository:'vendor_92021',state:'started',activation_authorized:false,
      available:true,ingress:false
    }})});
  });
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#count')?.textContent==='1');
  await page.getByRole('button',{name:'Détails de l’App'}).click();
  await page.waitForFunction(()=>document.querySelector('article.card .app-detail')?.textContent?.includes('vendor_92021'));
  if(detailCalls!==1)throw Error('Live details must only fetch upon explicit user action');
  if((await page.locator('article.card').innerText()).includes('MUST_NOT_LEAK'))throw Error('Unsafe details leaked');
  // New read-only live telemetry is fetched on demand, never from fixtures.
  let statsCalls=0;
  await page.route('**/api/apps/'+liveSlug+'/stats', async route=>{
   statsCalls++;
   await route.fulfill({status:200,contentType:'application/json',
    body:JSON.stringify({read_only:true,stats:{slug:liveSlug,read_only:true,
      cpu_percent:12.5,memory_usage:52428800,memory_limit:104857600,
      token:'MUST_NOT_LEAK'}})});
  });
  if(statsCalls!==0)throw Error('Stats must not fetch without user action');
  await page.getByRole('button',{name:'Ressources en direct'}).click();
  await page.waitForFunction(()=>document.querySelector('article.card .app-stats')?.textContent?.includes('CPU : 12.5 %'));
  if(statsCalls!==1)throw Error('Stats should fetch once on explicit action');
  if(!(await page.locator('article.card .app-stats').innerText()).includes('50.0 Mio / 100.0 Mio'))
   throw Error('Wrong numeric resource display');
  if((await page.locator('article.card').innerText()).includes('MUST_NOT_LEAK'))
   throw Error('Unsafe stats field leaked into dashboard');
  const entries=Array.from({length:125},(_,i)=>({
   slug:'community_new_'+i,repository:'unfamiliar_'+(i%17),
   name:'Future Community App '+String(i).padStart(3,'0'),
   available:true,installed:i%3===0,update_available:false
  }));
  entries[124].name='<img src=x onerror=alert(1)> New Third Party';
  let storeCalls=0;
  await page.route('**/api/store', async route=>{
   storeCalls++;
   await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({
    read_only:true,catalog:{official_supervisor_owner:true,
      activation_authorized:false,api_version:'V1',apps:entries,repositories:[]}
   })});
  });
  await page.locator('#load-store').click();
  await page.waitForFunction(()=>document.querySelector('#store-page')?.textContent?.includes('1 / 6'));
  if((await page.locator('#store-apps .card').count())!==24)throw Error('Store must paginate every vendor');
  await page.locator('#store-search').fill('Future Community App 063');
  if((await page.locator('#store-apps .card').count())!==1)throw Error('Store search across unknown Apps broken');
  await page.locator('#store-search').fill('<img');
  if((await page.locator('#store-apps .card').count())!==1)throw Error('Unsafe name filtered incorrectly');
  if((await page.locator('#store-apps img').count())!==0)throw Error('Store rendered untrusted HTML');
  if(storeCalls!==1)throw Error('Local Store filtering must not re-fetch catalog');
  let operationCalls=0;
  await page.route('**/api/store/apps/**/operations',async route=>{
   operationCalls++;
   if(route.request().method()!=='GET')throw Error('Operation preflight must never POST');
   const found=route.request().url().match(/\/api\/store\/apps\/([^/]+)\/operations$/);
   if(!found)throw Error('Unexpected operation route');
   const slug=decodeURIComponent(found[1]);
   const operations=['install','start','stop','restart','update','backup','restore'];
   await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({
    read_only:true,activation_authorized:false,slug,
    preflights:operations.map(action=>({
     slug,action,read_only:true,activation_authorized:false,
     platform_candidate:false,verified_backup_required:action==='update'||action==='restore',
     status:action==='update'?'verified_backup_required':'unknown_state'
    }))
   })});
  });
  await page.getByRole('button',{name:'Précontrôle des opérations'}).click();
  await page.waitForFunction(()=>document.querySelector('#store-apps .card')
      ?.textContent?.includes('Preuve de sauvegarde indispensable'));
  if(operationCalls!==1)throw Error('Operation evidence is not on-demand GET');
  const text=await page.locator('#store-apps .card').innerText();
  if(!text.includes('Aucune opération n’est autorisée')||!text.includes('Restaurer'))
   throw Error('UI must show seven read-only operations and no write permit');

  // Browser contract: official job discovery is admin-only, read-only,
  // dynamic and never renders private Supervisor job metadata.
  const discoveredId='0123456789abcdef0123456789abcdef';
  await page.unroute('**/api/session');
  await page.route('**/api/session',route=>route.fulfill({
   status:200,contentType:'application/json',body:JSON.stringify({
    authenticated:true,administrator:true,authority:'supervisor_ingress',
    actions_authorized:false,read_only:true
   })
  }));
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#load-jobs')?.disabled===false);
  let discoveryCalls=0,jobDetailCalls=0;
  await page.route('**/api/jobs',async route=>{
   discoveryCalls++;
   if(route.request().method()!=='GET')throw Error('Discovery must be GET-only');
   await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({
    read_only:true,truncated:false,jobs:[{
     id:discoveredId,progress:42,state:'running',error_count:0,read_only:true,
     extra:'NEVER_RENDER_PRIVATE_JOB_EXTRA'
    }]
   })});
  });
  await page.route('**/api/jobs/'+discoveredId,async route=>{
   jobDetailCalls++;
   await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({
    read_only:true,job:{id:discoveredId,progress:100,state:'succeeded',
     error_count:0,read_only:true}
   })});
  });
  await page.locator('#load-jobs').click();
  await page.waitForFunction(id=>document.querySelector('#job-list')?.textContent?.includes(id),discoveredId);
  if(discoveryCalls!==1)throw Error('Official job index should fetch once on demand');
  if((await page.locator('body').innerText()).includes('NEVER_RENDER_PRIVATE_JOB_EXTRA'))
   throw Error('Private job field leaked');
  await page.locator('#job-list button').first().click();
  await page.waitForFunction(()=>document.querySelector('#job-status')?.textContent?.includes('Terminé'));
  if(jobDetailCalls!==1||(await page.locator('#job-id').inputValue())!==discoveredId)
   throw Error('Discovered job was not linked to the existing official tracker');
  await page.unroute('**/api/session');
  await page.route('**/api/session',route=>route.fulfill({
   status:200,contentType:'application/json',body:JSON.stringify({
    authenticated:true,administrator:false,authority:'supervisor_ingress',
    actions_authorized:false,read_only:true
   })
  }));
  await page.locator('#refresh').click();
  await page.waitForFunction(()=>document.querySelector('#session-status')?.textContent?.includes('droits administrateur absents'));
  if(!(await page.locator('#load-jobs').isDisabled())||
     (await page.locator('#job-list button').count())!==0)
   throw Error('Non-admin browser retained access to discovered jobs');
  console.log('SUPERVISOR_JOBS_BROWSER_PASS: dynamic admin-only GET index, selection, redaction and revocation');

  console.log('SUPERVISOR_STORE_BROWSER_PASS: 125 arbitrary Apps, 24/page, search and safe rendering');
  console.log('LIVE_APP_DETAIL_BROWSER_PASS: unknown live App detail loaded on demand, no write action');
  console.log('LIVE_APP_STATS_BROWSER_PASS: on-demand read-only CPU/RAM and secret-field isolation');
  console.log('UNIVERSAL_DASHBOARD_E2E_PASS: 83 unknown Apps, 24/page, source/search, no unsafe HTML or unnecessary fetch');
  console.log('BRANDING_SCREENSHOTS_PASS: actual desktop and mobile HTML, 4 synthetic apps, zero production data');
 } finally { await browser.close(); }
} finally {await new Promise(r=>server.close(r));}
