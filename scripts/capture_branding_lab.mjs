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
 release_gates_total:15,release_gates_blocked:15,
 haos_apps_total:9,haos_apps_not_production_qualified:9 };
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
  if(!(await page.locator('#readiness').innerText()).includes('15 / 15'))throw Error('Synthetic readiness not visible');
  if((await page.locator('.card').count())!==4)throw Error('Unexpected synthetic app count');
  await page.screenshot({path:'branding/lab-dashboard.png',fullPage:true,animations:'disabled'});
  const mobile = await browser.newPage({viewport:{width:390,height:844},deviceScaleFactor:1});
  await mobile.goto('http://127.0.0.1:'+port+'/',{waitUntil:'load'});
  await mobile.waitForFunction(()=>document.querySelector('#status')?.textContent==='Inventaire disponible');
  if(!(await mobile.locator('#readiness').innerText()).includes('15 / 15'))throw Error('Mobile release gate state not visible');
  if((await mobile.locator('.card').count())!==4)throw Error('Mobile synthetic app count incorrect');
  await mobile.screenshot({path:'branding/lab-dashboard-mobile.png',fullPage:true,animations:'disabled'});
  console.log('BRANDING_SCREENSHOTS_PASS: actual desktop and mobile HTML, 4 synthetic apps, zero production data');
 } finally { await browser.close(); }
} finally {await new Promise(r=>server.close(r));}
