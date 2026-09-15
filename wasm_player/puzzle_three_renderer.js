function al(r,e){if(e instanceof AggregateError&&e.errors.length>0){for(let t of e.errors)al(r,t);return}r.push(e)}function $m(r){return r instanceof Error?r.message:String(r)}function rt(r,e){let t=[];for(let i of e)try{i()}catch(n){al(t,n)}if(t.length===1)throw t[0];if(t.length>1)throw new AggregateError(t,`${r} failed: ${$m(t[0])}`,{cause:t[0]})}var Hn=class extends Error{constructor(e){super(e),this.name="SurfaceRetryablePublicationError"}},Er=class extends Error{constructor(e){super(e),this.name="SurfaceFatalPublicationError"}},Io=class{#e;#i;#n;#t=new WeakSet;#o=[];#r=[];#s=null;#a=null;#c=null;#l=null;#d=0;#u=!1;constructor(e,t){this.#e=e,this.#i=t,this.#n=e.surface}get presented(){return this.#c}displaySample(e){return this.#h(),this.#e.prepare(e)}beginCommit(e){if(this.#h(),this.#t.has(e))throw new Error("prepared candidate has already been committed or discarded");this.#t.add(e),this.#s&&this.#e.disposePrepared(this.#s),this.#s=e,this.#f()}discard(e){this.#h(),!this.#t.has(e)&&(this.#t.add(e),this.#e.disposePrepared(e))}pollCommitted(){return this.#o.shift()??null}pollFailure(){return this.#r.shift()??null}supersedePending(){this.#h();let e=this.#s;this.#s=null,e&&this.#e.disposePrepared(e)}isSubmitted(e){return this.#a?.candidate===e}resetGeneration(e){this.#h(),rt(`surface presenter generation ${e} reset`,this.#m(t=>t===e))}resetSession(){this.#h(),rt("surface presenter session reset",this.#m(()=>!0))}dispose(){if(this.#u)return;this.#u=!0;let e=this.#m(()=>!0);e.push(()=>this.#e.dispose()),rt("surface presenter disposal",e)}#f(){if(this.#l!==null)return;let e=this.#d,t=0;t=this.#i.request(()=>{this.#u||e!==this.#d||this.#l!==t||this.#p()}),this.#l=t}#p(){if(this.#l=null,this.#u)return;let e=this.#e.contextState(),t=this.#e.surface===this.#n,i=this.#a;if(this.#a=null,i){let o=!1;if(t&&e.live&&e.epoch===i.contextEpoch)try{this.#e.confirmPublished(i.candidate),this.#c=i.metadata,this.#o.push(i.metadata)}catch(s){let a=s;try{this.#e.rollbackPublished(i.candidate)}catch(c){a=new Er(`publication confirmation failed: ${Wn(s)}; rollback failed: ${Wn(c)}`)}o=!0,this.#r.push(Object.freeze({generationId:i.metadata.generationId,candidateId:i.metadata.candidateId,fatal:la(a),message:Wn(a)}))}else try{this.#e.rollbackPublished(i.candidate),o=!0,this.#r.push(Object.freeze({generationId:i.metadata.generationId,candidateId:i.metadata.candidateId,fatal:!1,message:t?e.live?"rendering context epoch changed before commit receipt":"rendering context was lost before commit receipt":"public surface identity changed before commit receipt"}))}catch(s){o=!0,this.#r.push(Object.freeze({generationId:i.metadata.generationId,candidateId:i.metadata.candidateId,fatal:la(s),message:Wn(s)}))}try{this.#e.disposePrepared(i.candidate)}catch(s){o||this.#r.push(Object.freeze({generationId:i.metadata.generationId,candidateId:i.metadata.candidateId,fatal:!0,message:`submitted candidate cleanup failed: ${s instanceof Error?s.message:String(s)}`}))}}let n=this.#s;if(this.#s=null,n){if(t&&e.live)try{this.#a={metadata:Object.freeze({generationId:n.generationId,candidateId:n.requestId,layoutId:n.requestId,extent:n.extent,interactionRevision:n.interactionRevision,hitMap:n.hitMap}),contextEpoch:e.epoch,candidate:n},this.#f(),this.#e.publish(n);return}catch(o){this.#r.push(Object.freeze({generationId:n.generationId,candidateId:n.requestId,fatal:la(o),message:Wn(o)})),this.#a=null}else this.#r.push(Object.freeze({generationId:n.generationId,candidateId:n.requestId,fatal:!1,message:t?"rendering context was unavailable for publication":"public surface identity changed before publication"}));this.#e.disposePrepared(n)}(this.#s||this.#a)&&this.#f()}#h(){if(this.#u)throw new Error("surface presenter has been disposed")}#m(e){let t=[],i=this.#a&&e(this.#a.metadata.generationId)?this.#a:null,n=this.#s&&e(this.#s.generationId)?this.#s:null;if(i&&(this.#a=null),n&&(this.#s=null),this.#c&&e(this.#c.generationId)&&(this.#c=null),cl(this.#o,o=>e(o.generationId)),cl(this.#r,o=>e(o.generationId)),this.#l!==null&&this.#a===null&&this.#s===null){let o=this.#l;this.#l=null,this.#d+=1,t.push(()=>this.#i.cancel(o))}return i&&t.push(()=>this.#e.rollbackPublished(i.candidate)),n&&t.push(()=>this.#e.disposePrepared(n)),i&&t.push(()=>this.#e.disposePrepared(i.candidate)),(this.#s||this.#a)&&this.#l===null&&t.push(()=>this.#f()),t}};function la(r){return!(r instanceof Hn)}function Wn(r){return r instanceof Error?r.message:String(r)}function cl(r,e){let t=0;for(let i of r)e(i)||(r[t]=i,t+=1);r.length=t}var Do=class{request(e){return globalThis.requestAnimationFrame(()=>e())}cancel(e){globalThis.cancelAnimationFrame(e)}};var Ym=18446744073709551615n,Km=/^(?:0|[1-9][0-9]*)$/,Ze=class extends Error{code;path;constructor(e,t,i){super(i),this.code=e,this.path=t}};function Pi(r,e,t,i=[]){if(r===null||typeof r!="object"||Array.isArray(r))throw new Ze("invalid_schema",e,`${e} must be an object.`);if(Object.getOwnPropertySymbols(r).length!==0)throw new Ze("invalid_schema",e,`${e} must not contain symbol fields.`);let n=r,o=new Set([...t,...i]);for(let s of Object.getOwnPropertyNames(n)){if(!o.has(s))throw new Ze("invalid_schema",`${e}.${s}`,`${e} contains unknown field ${JSON.stringify(s)}.`);let a=Object.getOwnPropertyDescriptor(n,s);if(!a||!("value"in a)||!a.enumerable)throw new Ze("invalid_schema",`${e}.${s}`,`${e}.${s} must be an enumerable data field.`)}for(let s of t)if(!Object.hasOwn(n,s))throw new Ze("invalid_schema",`${e}.${s}`,`${e} is missing required field ${JSON.stringify(s)}.`);return n}function Xn(r,e){if(!Array.isArray(r))throw new Ze("invalid_schema",e,`${e} must be an array.`);if(Object.getOwnPropertySymbols(r).length!==0)throw new Ze("invalid_schema",e,`${e} must not contain symbol fields.`);let t=new Set(["length"]);for(let i=0;i<r.length;i+=1)t.add(i.toString());for(let i of Object.getOwnPropertyNames(r))if(!t.has(i))throw new Ze("invalid_schema",`${e}.${i}`,`${e} contains an unknown array field.`);for(let i=0;i<r.length;i+=1){let n=Object.getOwnPropertyDescriptor(r,i.toString());if(!n||!("value"in n)||!n.enumerable)throw new Ze("invalid_schema",`${e}[${i}]`,`${e}[${i}] must be an enumerable data item.`)}return r}function Gt(r,e){if(typeof r!="string"||!Km.test(r))throw new Ze("non_canonical_u64",e,`${e} must be a canonical unsigned decimal string.`);let t=BigInt(r);if(t>Ym)throw new Ze("non_canonical_u64",e,`${e} exceeds the u64 range.`);return t}function dl(r,e,t){if(r!==e)throw new Ze("invalid_schema",t,`${t} must be ${JSON.stringify(e)}.`);return e}function qn(r,e){if(typeof r!="string")throw new Ze("invalid_schema",e,`${e} must be a string.`);return r}function jn(r){if(typeof r=="bigint")return`{"$u64":${JSON.stringify(r.toString())}}`;if(r===null||typeof r=="boolean"||typeof r=="string")return JSON.stringify(r);if(typeof r=="number"){if(!Number.isFinite(r))throw new Error("A validated wire value contains a non-finite number.");return Object.is(r,-0)?"-0":JSON.stringify(r)}if(Array.isArray(r))return`[${r.map(e=>jn(e)).join(",")}]`;if(typeof r=="object"){let e=r;return`{${Object.keys(e).sort().map(t=>`${JSON.stringify(t)}:${jn(e[t])}`).join(",")}}`}throw new Error("A validated wire value contains an unsupported JavaScript type.")}function cr(r,e){if(Object.is(r,e))return!0;if(typeof r!=typeof e||r===null||e===null)return!1;if(Array.isArray(r))return!Array.isArray(e)||r.length!==e.length?!1:r.every((s,a)=>cr(s,e[a]));if(Array.isArray(e)||typeof r!="object"||typeof e!="object")return!1;let t=r,i=e,n=Object.keys(t),o=Object.keys(i);return n.length!==o.length?!1:n.every(s=>Object.hasOwn(i,s)&&cr(t[s],i[s]))}function Lo(r,e="apply:v1"){let t=jn(r);return Zm(`puzzlestudio:browser-live-surface:${e}\0${t}`)}async function ul(r,e="apply:v1",t=4){await ll();let i=[],n=performance.now();for(let c of da(r))i.push(c),performance.now()-n>=t&&(await ll(),n=performance.now());let o=i.join(""),s=new TextEncoder().encode(`puzzlestudio:browser-live-surface:${e}\0${o}`),a=await globalThis.crypto.subtle.digest("SHA-256",s);return[...new Uint8Array(a)].map(c=>c.toString(16).padStart(2,"0")).join("")}function*da(r){if(typeof r=="bigint"){yield`{"$u64":${JSON.stringify(r.toString())}}`;return}if(r===null||typeof r=="boolean"||typeof r=="string"){yield JSON.stringify(r);return}if(typeof r=="number"){if(!Number.isFinite(r))throw new Error("A validated wire value contains a non-finite number.");yield Object.is(r,-0)?"-0":JSON.stringify(r);return}if(Array.isArray(r)){yield"[";for(let e=0;e<r.length;e+=1)e!==0&&(yield","),yield*da(r[e]);yield"]";return}if(typeof r=="object"){let e=r;yield"{";let t=Object.keys(e).sort();for(let i=0;i<t.length;i+=1){i!==0&&(yield",");let n=t[i];yield JSON.stringify(n),yield":",yield*da(e[n])}yield"}";return}throw new Error("A validated wire value contains an unsupported JavaScript type.")}function ll(){return new Promise(r=>setTimeout(r,0))}function Zm(r){let e=new TextEncoder().encode(r),t=Math.ceil((e.length+9)/64)*64,i=new Uint8Array(t);i.set(e),i[e.length]=128;let n=e.length*8,o=new DataView(i.buffer);o.setUint32(t-8,Math.floor(n/4294967296),!1),o.setUint32(t-4,n>>>0,!1);let s=new Uint32Array([1779033703,3144134277,1013904242,2773480762,1359893119,2600822924,528734635,1541459225]),a=new Uint32Array(64);for(let c=0;c<t;c+=64){for(let f=0;f<16;f+=1)a[f]=o.getUint32(c+f*4,!1);for(let f=16;f<64;f+=1){let S=a[f-15],_=a[f-2],R=Ci(S,7)^Ci(S,18)^S>>>3,P=Ci(_,17)^Ci(_,19)^_>>>10;a[f]=a[f-16]+R+a[f-7]+P>>>0}let[l,d,u,h,p,x,g,m]=s;for(let f=0;f<64;f+=1){let S=Ci(p,6)^Ci(p,11)^Ci(p,25),_=p&x^~p&g,R=m+S+_+Jm[f]+a[f]>>>0,P=Ci(l,2)^Ci(l,13)^Ci(l,22),w=l&d^l&u^d&u,T=P+w>>>0;m=g,g=x,x=p,p=h+R>>>0,h=u,u=d,d=l,l=R+T>>>0}s[0]=s[0]+l>>>0,s[1]=s[1]+d>>>0,s[2]=s[2]+u>>>0,s[3]=s[3]+h>>>0,s[4]=s[4]+p>>>0,s[5]=s[5]+x>>>0,s[6]=s[6]+g>>>0,s[7]=s[7]+m>>>0}return[...s].map(c=>c.toString(16).padStart(8,"0")).join("")}function Ci(r,e){return r>>>e|r<<32-e}var Jm=Object.freeze([1116352408,1899447441,3049323471,3921009573,961987163,1508970993,2453635748,2870763221,3624381080,310598401,607225278,1426881987,1925078388,2162078206,2614888103,3248222580,3835390401,4022224774,264347078,604807628,770255983,1249150122,1555081692,1996064986,2554220882,2821834349,2952996808,3210313671,3336571891,3584528711,113926993,338241895,666307205,773529912,1294757372,1396182291,1695183700,1986661051,2177026350,2456956037,2730485921,2820302411,3259730800,3345764771,3516065817,3600352804,4094571909,275423344,430227734,506948616,659060556,883997877,958139571,1322822218,1537002063,1747873779,1955562222,2024104815,2227730452,2361852424,2428436474,2756734187,3204031479,3329325298]);var et=Object.freeze({kind:"string"}),qt=Object.freeze({kind:"boolean"}),Z=Object.freeze({kind:"finite"}),dt=Object.freeze({kind:"u64"}),Qm=Yn(0,255),Nt=Yn(0,65535),Vi=Yn(0,4294967295),$n=Yn(-32768,32767),zi=Yn(-2147483648,2147483647);function Yn(r,e){return Object.freeze({kind:"integer",minimum:r,maximum:e})}function Ar(r){return Object.freeze({kind:"literal",value:r})}function Rt(r){return Object.freeze({kind:"nullable",value:r})}function We(r){return Object.freeze({kind:"array",item:r})}function Lt(...r){return Object.freeze({kind:"tuple",items:r})}function ue(r){return Object.freeze({kind:"record",fields:Object.freeze(r)})}function Jt(r,e="kind",t){let i=Object.freeze(r);return Object.freeze(t?{kind:"tagged",tag:e,variants:i,refine:t}:{kind:"tagged",tag:e,variants:i})}function xt(...r){return Jt(Object.fromEntries(r.map(e=>[e,{}])),"__literal")}var gt=ue({red:Z,green:Z,blue:Z,alpha:Z}),ha=ue({x:Z,y:Z,width:Z,height:Z}),fa=ue({color:gt,offsetXPx:Z,offsetYPx:Z,blurPx:Z}),yl=ue({id:et,revision:et}),Sl=Jt({uniform:{},graphite:{roughness:Z,pressureVariation:Z,overdraw:Qm,seed:Vi}}),bl=ue({sampling:xt("smooth","nearest"),pixelSnapping:qt,shadow:fa,outline:gt,outlineWidthPx:Z,postEffect:xt("none","graphite","scanlines","vignette","stage_light"),postEffectStrength:Z}),Uo=ue({fontSizePx:Z,lineHeight:Z,maxWidthPx:Rt(Z),alignment:xt("start","center","end")}),hl=ue({fill:gt,border:gt,borderWidthPx:Z,cornerRadiusPx:Z,shadow:fa}),eg=ue({paddingHorizontalPx:Z,paddingVerticalPx:Z,marginPx:Z,borderWidthPx:Z,cornerRadiusPx:Z,widthPx:Rt(Z),selectionMarker:Rt(ue({idleGlyph:et,selectedGlyph:et,selectedFill:qt,columnGapPx:Z}))}),ua=ue({kind:xt("fill","outline","glyph","lift","spotlight"),foreground:gt,fill:gt,border:gt,liftPx:Z,sketch:Rt(ue({underlineStrokePx:Z,outlineStrokePx:Z,roughness:Z}))}),Oo=ue({theme:ue({font:yl,typography:ue({heading:Uo,subheading:Uo,body:Uo,caption:Uo}),uiSkin:ue({canvas:gt,text:gt,mutedText:gt,accent:gt,panel:hl,modal:hl,control:ue({fill:gt,text:gt,border:gt,layout:eg}),statusMarks:ue({cleared:et}),layout:ue({sceneGapPx:Z,containerGapPx:Z,modalMaxWidthPx:Z,modalPaddingPx:Z})}),backdropTreatment:Jt({none:{},paper:{grain:ue({scalePx:Z,strength:Z,erasureCount:Nt,erasureScalePx:Lt(Z,Z)}),ruling:ue({kind:xt("none","ruled","grid"),spacingPx:Lt(Z,Z),lineWidthPx:Z,color:gt})}}),boardSkin:ue({frame:ue({border:gt,borderWidthPx:Z,cornerRadiusPx:Z,shadow:fa}),grid:gt,gridStroke:Sl,viewportPaddingPx:Z}),interactionInk:ue({focus:ua,selected:ua,target:ua,disabledOpacity:Z}),renderTreatment:bl,uiReferenceSize:ue({widthPx:Z,heightPx:Z})})}),No=Jt({font:{id:et,revision:et},image:{id:et,revision:et}}),Ii=ue({session:dt,stateCommit:dt}),Xr=ue({component:et,treePath:We(Vi)}),qr=ue({model:et,component:et,source:et}),tg=ue({space:Jt({fit:{},fill:{weight:Nt}}),scroll:qt,alignSelf:Rt(xt("start","center","end","stretch")),aspectRatio:Rt(ue({width:Nt,height:Nt})),gap:Rt(Nt),align:xt("start","center","end","stretch"),distribute:xt("start","center","end","between")}),fl=ue({id:ue({component:et,ordinal:Vi}),activation:Jt({idle:{},awaiting_publication:{activationId:dt,sessionRevision:dt},activated:{}}),enabled:qt}),ig=Jt({viewport:{viewport:qr},frame:{frameKind:et,source:et},text:{role:xt("heading","subheading","body","caption"),value:et,textAlign:Rt(xt("start","center","end"))},button:{label:et,control:fl,selected:qt},toggle:{label:et,checked:qt,control:fl,selected:qt},audio_controls:{},row:{children:We(Xr)},column:{children:We(Xr)},box:{children:We(Xr)},error:{message:et}}),Rl=ue({id:Xr,layout:tg,content:ig}),Bo=ue({rootComponent:Rt(et),focusComponent:et,components:We(ue({id:et,placement:xt("root","content","overlay"),visibility:xt("visible","hidden"),modal:qt,modalSurface:qt,rootNode:Xr})),nodes:We(Rl)}),rg=ue({projection:xt("perspective","orthographic"),yawDegrees:$n,pitchDegrees:$n,rollDegrees:$n,zoom:Z,framing:Rt(ue({center:Lt(Z,Z,Z),size:Lt(Z,Z,Z)})),interactiveLook:qt,interactiveZoom:qt}),Ml=Jt({two_d:{origin:Lt(zi,zi),size:Lt(Nt,Nt),displayError:Rt(et)},three_d:{camera:rg,lighting:ue({intensity:Z,ambient:Z,yawDegrees:$n,pitchDegrees:$n,color:gt}),shade:qt,shadow:qt,pixelate:ue({enabled:qt,scale:Nt,smoothing:qt}),renderTreatment:bl}}),wl=ue({renderOrder:dt,objectIds:We(Nt),visualIds:We(Nt),instanceIds:We(dt),cell:Lt(zi,zi,zi),contentKind:xt("pixels","voxels","raster_image","text"),occurrence:Vi}),ng=ue({position:Lt(zi,zi,zi),color:gt}),og=Jt({pixels:{width:Nt,height:Nt,positions:We(zi),palette:We(Z),paletteIndices:We(Vi)},voxels:{width:Nt,depth:Nt,height:Nt,voxels:We(ng)},raster_image:{asset:et,revision:et,frame:Vi,sourceSize:Lt(Nt,Nt),destination:ha,uv:ha,sampling:xt("pixelated","smooth")},text:{value:et,font:yl,fontSize:Z,align:xt("top_left","top","top_right","left","center","right","bottom_left","bottom","bottom_right"),overflow:xt("clip","visible"),color:gt}},"kind",vg),sg=ue({x:Z,y:Z,width:Z,height:Z,clip:Rt(ha)}),pa=ue({key:wl,drawIndex:Vi,transform:Lt(...Array.from({length:16},()=>Z)),opacity:Z,pixelGeometry:Rt(sg),content:og}),pl=ue({color:gt,width:Jt({cell_relative:{cellFraction:Z,minPhysicalPixels:Z},physical_pixels:{pixels:Z}}),treatment:Sl}),ma=Jt({lines_2d:{segments:We(ue({start:Lt(Z,Z),end:Lt(Z,Z)})),style:pl},lines_3d:{segments:We(ue({start:Lt(Z,Z,Z),end:Lt(Z,Z,Z)})),style:pl,depth:xt("tested","overlay")},triangles_3d:{triangles:We(ue({points:Lt(Lt(Z,Z,Z),Lt(Z,Z,Z),Lt(Z,Z,Z))})),color:gt,depth:xt("tested","overlay")}}),ga=ue({id:qr,projection:Ml,batches:We(pa),decorations:We(ma)}),Tl=ue({revision:Ii,configuration:Oo,resources:We(No),surface:Bo,viewports:We(ga)}),ml=ue({removeResources:We(No),upsertResources:We(No)}),ag=Jt({stable:{baseRevision:Ii,targetRevision:Ii,resources:ml,targetConfiguration:Rt(Oo),targetSurface:Rt(Bo),changedViewports:We(qr)},reconfigure:{baseRevision:Ii,targetRevision:Ii,resources:ml,targetConfiguration:Oo,targetSurface:Bo,retainedViewports:We(qr),replacementViewports:We(ga)}}),cg=Jt({initial:{},timeline:{block:dt,localMilliseconds:dt},steady:{}}),lg=ue({nodeId:Xr,replacement:Rt(Rl)}),dg=ue({viewport:qr,projection:Rt(Ml),removeBatchKeys:We(wl),upsertBatches:We(pa),targetDecorations:Rt(We(ma))}),ug=ue({version:Ar(1),generationId:dt,target:Tl}),hg=ue({version:Ar(1),generationId:dt,transactionId:Ii,baseRevision:Ii,targetRevision:Ii,target:Tl,diff:ag}),fg=ue({version:Ar(1),generationId:dt,requestId:dt,targetRevision:Ii,position:cg,clipMilliseconds:dt,extent:ue({width:Vi,height:Vi}),interactionRevision:dt,configurationOverride:Rt(Oo),resourceBindings:We(No),surfaceOverride:Rt(Bo),nodePatches:We(lg),removeViewports:We(qr),upsertViewports:We(ga),viewportPatches:We(dg)}),Fo=ue({x:Z,y:Z}),gl=ue({x:Z,y:Z}),pg=ue({version:Ar(1),generationId:dt,layoutId:dt,input:Jt({press:{pointer:dt,point:Fo,button:xt("primary","middle")},move:{pointer:dt,point:Fo,delta:gl},release:{pointer:dt,point:Fo,button:xt("primary","middle")},cancel:{pointer:dt},wheel:{point:Fo,delta:gl,unit:xt("line","pixel")}})}),mg=ue({version:Ar(1),generationId:dt,candidateId:dt}),gg=ue({version:Ar(1),generationId:dt}),xg=ue({version:Ar(1),generationId:dt,revisions:We(Ii)});function El(r){let e=Gi(r,ug),t=e.target;return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),targetRevision:t.revision,target:t})}function Al(r){let e=Gi(r,hg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),transactionId:e.transactionId,baseRevision:e.baseRevision,targetRevision:e.targetRevision,target:e.target,diff:e.diff})}function Cl(r){let e=Gi(r,fg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),requestId:Gt(e.requestId,"$.requestId"),targetRevision:e.targetRevision,sample:e})}function Pl(r){let e=Gi(r,pg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),layoutId:Gt(e.layoutId,"$.layoutId"),input:e.input})}function Il(r){let e=Gi(r,mg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),candidateId:Gt(e.candidateId,"$.candidateId")})}function ko(r){let e=Gi(r,gg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId")})}function Dl(r){let e=Gi(r,xg);return Object.freeze({generationId:Gt(e.generationId,"$.generationId"),revisions:e.revisions})}function mi(r){return`${r.session.length}:${r.session}:${r.stateCommit}`}function Pe(r){return jn(r)}function Gi(r,e){return jr(r,e,"$")}function jr(r,e,t){switch(e.kind){case"array":return xl(r,t,(i,n)=>jr(i,e.item,`${t}[${n}]`));case"boolean":if(typeof r!="boolean")throw Je(t,`${t} must be boolean`);return r;case"finite":if(typeof r!="number"||!Number.isFinite(r))throw Je(t,`${t} must be finite`);return r;case"integer":if(typeof r!="number"||!Number.isInteger(r)||r<e.minimum||r>e.maximum)throw Je(t,`${t} must be an integer in [${e.minimum}, ${e.maximum}]`);return r;case"literal":if(r!==e.value)throw Je(t,`${t} must equal ${JSON.stringify(e.value)}`);return r;case"nullable":return r==null?null:jr(r,e.value,t);case"record":{let i=Object.keys(e.fields),n=vl(r,t);_l(n,i,t);let o={};for(let s of i)o[s]=jr(n.get(s),e.fields[s],`${t}.${s}`);return Object.freeze(o)}case"string":if(typeof r!="string")throw Je(t,`${t} must be a string`);return r;case"tagged":{if(e.tag==="__literal"){if(typeof r!="string"||!Object.hasOwn(e.variants,r))throw Je(t,`${t} has an unsupported enum value`);return r}let i=vl(r,t),n=i.get(e.tag);if(typeof n!="string"||!Object.hasOwn(e.variants,n))throw Je(`${t}.${e.tag}`,`${t}.${e.tag} has an unsupported variant`);let o=e.variants[n],s=[e.tag,...Object.keys(o)];_l(i,s,t);let a={[e.tag]:n};for(let[c,l]of Object.entries(o))a[c]=jr(i.get(c),l,`${t}.${c}`);return e.refine?.(a,n,t),Object.freeze(a)}case"tuple":return xl(r,t,(i,n)=>jr(i,e.items[n],`${t}[${n}]`),e.items.length);case"u64":return Gt(r,t),r}}function vg(r,e,t){if(e!=="pixels")return;let i=r.positions,n=r.palette,o=r.paletteIndices;if(i.length%2!==0)throw Je(`${t}.positions`,`${t}.positions must contain complete x/y pairs`);if(n.length%4!==0)throw Je(`${t}.palette`,`${t}.palette must contain complete linear-RGBA quads`);if(i.length/2!==o.length)throw Je(`${t}.paletteIndices`,`${t}.paletteIndices must contain exactly one index per position pair`);let s=n.length/4;if(o.length===0&&s!==0)throw Je(`${t}.palette`,`${t}.palette cannot contain unused entries`);if(o.length!==0&&s===0)throw Je(`${t}.palette`,`${t}.palette is required when pixels are present`);let a=new Set;for(let d=0;d<s;d+=1){let h=n.slice(d*4,d*4+4).map(p=>Object.is(p,-0)?"-0":p.toString()).join(",");if(a.has(h))throw Je(`${t}.palette`,`${t}.palette contains a duplicate bit-exact color`);a.add(h)}let c=new Set,l=0;for(let d=0;d<o.length;d+=1){let u=o[d];if(u>=s)throw Je(`${t}.paletteIndices[${d}]`,`${t}.paletteIndices[${d}] is outside the palette`);if(!c.has(u)){if(u!==l)throw Je(`${t}.paletteIndices[${d}]`,`${t}.paletteIndices must introduce palette entries in first-occurrence order`);c.add(u),l+=1}}if(l!==s)throw Je(`${t}.palette`,`${t}.palette cannot contain unused entries`)}function xl(r,e,t,i){if(!Array.isArray(r))throw Je(e,`${e} must be an array`);if(Object.getOwnPropertySymbols(r).length!==0)throw Je(e,`${e} contains an unknown symbol field`);let n=Object.getOwnPropertyDescriptor(r,"length");if(!n||!("value"in n)||n.enumerable||!Number.isSafeInteger(n.value)||n.value<0)throw Je(`${e}.length`,`${e}.length must be the array length data field`);let o=n.value;if(i!==void 0&&o!==i)throw Je(e,`${e} must contain exactly ${i} items`);let s=new Array(o),a=0;for(let c of Object.getOwnPropertyNames(r)){if(c==="length")continue;let l=Number(c);if(!Number.isInteger(l)||l<0||l>=o||l.toString()!==c)throw Je(`${e}.${c}`,`${e} contains an unknown array field`);let d=Object.getOwnPropertyDescriptor(r,c);if(!d||!("value"in d)||!d.enumerable)throw Je(`${e}[${l}]`,`${e}[${l}] must be an enumerable data item`);s[l]=t(d.value,l),a+=1}if(a!==o){for(let c=0;c<o;c+=1)if(!Object.hasOwn(s,c))throw Je(`${e}[${c}]`,`${e}[${c}] must be an enumerable data item`);throw Je(e,`${e} must be a dense array`)}return Object.freeze(s)}function vl(r,e){if(r===null||typeof r!="object"||Array.isArray(r))throw Je(e,`${e} must be an object`);if(Object.getOwnPropertySymbols(r).length!==0)throw Je(e,`${e} contains an unknown symbol field`);let t=new Map;for(let i of Object.getOwnPropertyNames(r)){let n=Object.getOwnPropertyDescriptor(r,i);if(!n||!("value"in n)||!n.enumerable)throw Je(`${e}.${i}`,`${e}.${i} must be an enumerable data field`);t.set(i,n.value)}return t}function _l(r,e,t){let i=new Set(e);for(let n of r.keys())if(!i.has(n))throw Je(`${t}.${n}`,`${t} contains unknown field ${JSON.stringify(n)}`);for(let n of e)if(!r.has(n))throw Je(`${t}.${n}`,`${t} is missing required field ${JSON.stringify(n)}`)}function Je(r,e){return new Ze("invalid_schema",r,e)}function Ll(r){return Gi(r,pa)}function Ul(r){return Gi(r,ma)}function lr(r,e){let t=Math.max(r.x,e.x),i=Math.max(r.y,e.y),n=Math.min(r.x+r.width,e.x+e.width),o=Math.min(r.y+r.height,e.y+e.height);return n>t&&o>i?Object.freeze({x:t,y:i,width:n-t,height:o-i}):null}var Wt=["00","01","02","03","04","05","06","07","08","09","0a","0b","0c","0d","0e","0f","10","11","12","13","14","15","16","17","18","19","1a","1b","1c","1d","1e","1f","20","21","22","23","24","25","26","27","28","29","2a","2b","2c","2d","2e","2f","30","31","32","33","34","35","36","37","38","39","3a","3b","3c","3d","3e","3f","40","41","42","43","44","45","46","47","48","49","4a","4b","4c","4d","4e","4f","50","51","52","53","54","55","56","57","58","59","5a","5b","5c","5d","5e","5f","60","61","62","63","64","65","66","67","68","69","6a","6b","6c","6d","6e","6f","70","71","72","73","74","75","76","77","78","79","7a","7b","7c","7d","7e","7f","80","81","82","83","84","85","86","87","88","89","8a","8b","8c","8d","8e","8f","90","91","92","93","94","95","96","97","98","99","9a","9b","9c","9d","9e","9f","a0","a1","a2","a3","a4","a5","a6","a7","a8","a9","aa","ab","ac","ad","ae","af","b0","b1","b2","b3","b4","b5","b6","b7","b8","b9","ba","bb","bc","bd","be","bf","c0","c1","c2","c3","c4","c5","c6","c7","c8","c9","ca","cb","cc","cd","ce","cf","d0","d1","d2","d3","d4","d5","d6","d7","d8","d9","da","db","dc","dd","de","df","e0","e1","e2","e3","e4","e5","e6","e7","e8","e9","ea","eb","ec","ed","ee","ef","f0","f1","f2","f3","f4","f5","f6","f7","f8","f9","fa","fb","fc","fd","fe","ff"];var zo=Math.PI/180,Kn=180/Math.PI;function Di(){let r=Math.random()*4294967295|0,e=Math.random()*4294967295|0,t=Math.random()*4294967295|0,i=Math.random()*4294967295|0;return(Wt[r&255]+Wt[r>>8&255]+Wt[r>>16&255]+Wt[r>>24&255]+"-"+Wt[e&255]+Wt[e>>8&255]+"-"+Wt[e>>16&15|64]+Wt[e>>24&255]+"-"+Wt[t&63|128]+Wt[t>>8&255]+"-"+Wt[t>>16&255]+Wt[t>>24&255]+Wt[i&255]+Wt[i>>8&255]+Wt[i>>16&255]+Wt[i>>24&255]).toLowerCase()}function Et(r,e,t){return Math.max(e,Math.min(t,r))}function Fl(r,e){return(r%e+e)%e}function Vo(r,e,t){return(1-t)*r+t*e}function $r(r,e){switch(e.constructor){case Float32Array:return r;case Uint32Array:return r/4294967295;case Uint16Array:return r/65535;case Uint8Array:return r/255;case Int32Array:return Math.max(r/2147483647,-1);case Int16Array:return Math.max(r/32767,-1);case Int8Array:return Math.max(r/127,-1);default:throw new Error("Invalid component type.")}}function $t(r,e){switch(e.constructor){case Float32Array:return r;case Uint32Array:return Math.round(r*4294967295);case Uint16Array:return Math.round(r*65535);case Uint8Array:return Math.round(r*255);case Int32Array:return Math.round(r*2147483647);case Int16Array:return Math.round(r*32767);case Int8Array:return Math.round(r*127);default:throw new Error("Invalid component type.")}}var Li=class{constructor(e=0,t=0,i=0,n=1){this.isQuaternion=!0,this._x=e,this._y=t,this._z=i,this._w=n}static slerpFlat(e,t,i,n,o,s,a){let c=i[n+0],l=i[n+1],d=i[n+2],u=i[n+3],h=o[s+0],p=o[s+1],x=o[s+2],g=o[s+3];if(a===0){e[t+0]=c,e[t+1]=l,e[t+2]=d,e[t+3]=u;return}if(a===1){e[t+0]=h,e[t+1]=p,e[t+2]=x,e[t+3]=g;return}if(u!==g||c!==h||l!==p||d!==x){let m=1-a,f=c*h+l*p+d*x+u*g,S=f>=0?1:-1,_=1-f*f;if(_>Number.EPSILON){let P=Math.sqrt(_),w=Math.atan2(P,f*S);m=Math.sin(m*w)/P,a=Math.sin(a*w)/P}let R=a*S;if(c=c*m+h*R,l=l*m+p*R,d=d*m+x*R,u=u*m+g*R,m===1-a){let P=1/Math.sqrt(c*c+l*l+d*d+u*u);c*=P,l*=P,d*=P,u*=P}}e[t]=c,e[t+1]=l,e[t+2]=d,e[t+3]=u}static multiplyQuaternionsFlat(e,t,i,n,o,s){let a=i[n],c=i[n+1],l=i[n+2],d=i[n+3],u=o[s],h=o[s+1],p=o[s+2],x=o[s+3];return e[t]=a*x+d*u+c*p-l*h,e[t+1]=c*x+d*h+l*u-a*p,e[t+2]=l*x+d*p+a*h-c*u,e[t+3]=d*x-a*u-c*h-l*p,e}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get w(){return this._w}set w(e){this._w=e,this._onChangeCallback()}set(e,t,i,n){return this._x=e,this._y=t,this._z=i,this._w=n,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._w)}copy(e){return this._x=e.x,this._y=e.y,this._z=e.z,this._w=e.w,this._onChangeCallback(),this}setFromEuler(e,t=!0){let i=e._x,n=e._y,o=e._z,s=e._order,a=Math.cos,c=Math.sin,l=a(i/2),d=a(n/2),u=a(o/2),h=c(i/2),p=c(n/2),x=c(o/2);switch(s){case"XYZ":this._x=h*d*u+l*p*x,this._y=l*p*u-h*d*x,this._z=l*d*x+h*p*u,this._w=l*d*u-h*p*x;break;case"YXZ":this._x=h*d*u+l*p*x,this._y=l*p*u-h*d*x,this._z=l*d*x-h*p*u,this._w=l*d*u+h*p*x;break;case"ZXY":this._x=h*d*u-l*p*x,this._y=l*p*u+h*d*x,this._z=l*d*x+h*p*u,this._w=l*d*u-h*p*x;break;case"ZYX":this._x=h*d*u-l*p*x,this._y=l*p*u+h*d*x,this._z=l*d*x-h*p*u,this._w=l*d*u+h*p*x;break;case"YZX":this._x=h*d*u+l*p*x,this._y=l*p*u+h*d*x,this._z=l*d*x-h*p*u,this._w=l*d*u-h*p*x;break;case"XZY":this._x=h*d*u-l*p*x,this._y=l*p*u-h*d*x,this._z=l*d*x+h*p*u,this._w=l*d*u+h*p*x;break;default:console.warn("THREE.Quaternion: .setFromEuler() encountered an unknown order: "+s)}return t===!0&&this._onChangeCallback(),this}setFromAxisAngle(e,t){let i=t/2,n=Math.sin(i);return this._x=e.x*n,this._y=e.y*n,this._z=e.z*n,this._w=Math.cos(i),this._onChangeCallback(),this}setFromRotationMatrix(e){let t=e.elements,i=t[0],n=t[4],o=t[8],s=t[1],a=t[5],c=t[9],l=t[2],d=t[6],u=t[10],h=i+a+u;if(h>0){let p=.5/Math.sqrt(h+1);this._w=.25/p,this._x=(d-c)*p,this._y=(o-l)*p,this._z=(s-n)*p}else if(i>a&&i>u){let p=2*Math.sqrt(1+i-a-u);this._w=(d-c)/p,this._x=.25*p,this._y=(n+s)/p,this._z=(o+l)/p}else if(a>u){let p=2*Math.sqrt(1+a-i-u);this._w=(o-l)/p,this._x=(n+s)/p,this._y=.25*p,this._z=(c+d)/p}else{let p=2*Math.sqrt(1+u-i-a);this._w=(s-n)/p,this._x=(o+l)/p,this._y=(c+d)/p,this._z=.25*p}return this._onChangeCallback(),this}setFromUnitVectors(e,t){let i=e.dot(t)+1;return i<Number.EPSILON?(i=0,Math.abs(e.x)>Math.abs(e.z)?(this._x=-e.y,this._y=e.x,this._z=0,this._w=i):(this._x=0,this._y=-e.z,this._z=e.y,this._w=i)):(this._x=e.y*t.z-e.z*t.y,this._y=e.z*t.x-e.x*t.z,this._z=e.x*t.y-e.y*t.x,this._w=i),this.normalize()}angleTo(e){return 2*Math.acos(Math.abs(Et(this.dot(e),-1,1)))}rotateTowards(e,t){let i=this.angleTo(e);if(i===0)return this;let n=Math.min(1,t/i);return this.slerp(e,n),this}identity(){return this.set(0,0,0,1)}invert(){return this.conjugate()}conjugate(){return this._x*=-1,this._y*=-1,this._z*=-1,this._onChangeCallback(),this}dot(e){return this._x*e._x+this._y*e._y+this._z*e._z+this._w*e._w}lengthSq(){return this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w}length(){return Math.sqrt(this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w)}normalize(){let e=this.length();return e===0?(this._x=0,this._y=0,this._z=0,this._w=1):(e=1/e,this._x=this._x*e,this._y=this._y*e,this._z=this._z*e,this._w=this._w*e),this._onChangeCallback(),this}multiply(e){return this.multiplyQuaternions(this,e)}premultiply(e){return this.multiplyQuaternions(e,this)}multiplyQuaternions(e,t){let i=e._x,n=e._y,o=e._z,s=e._w,a=t._x,c=t._y,l=t._z,d=t._w;return this._x=i*d+s*a+n*l-o*c,this._y=n*d+s*c+o*a-i*l,this._z=o*d+s*l+i*c-n*a,this._w=s*d-i*a-n*c-o*l,this._onChangeCallback(),this}slerp(e,t){if(t===0)return this;if(t===1)return this.copy(e);let i=this._x,n=this._y,o=this._z,s=this._w,a=s*e._w+i*e._x+n*e._y+o*e._z;if(a<0?(this._w=-e._w,this._x=-e._x,this._y=-e._y,this._z=-e._z,a=-a):this.copy(e),a>=1)return this._w=s,this._x=i,this._y=n,this._z=o,this;let c=1-a*a;if(c<=Number.EPSILON){let p=1-t;return this._w=p*s+t*this._w,this._x=p*i+t*this._x,this._y=p*n+t*this._y,this._z=p*o+t*this._z,this.normalize(),this}let l=Math.sqrt(c),d=Math.atan2(l,a),u=Math.sin((1-t)*d)/l,h=Math.sin(t*d)/l;return this._w=s*u+this._w*h,this._x=i*u+this._x*h,this._y=n*u+this._y*h,this._z=o*u+this._z*h,this._onChangeCallback(),this}slerpQuaternions(e,t,i){return this.copy(e).slerp(t,i)}random(){let e=2*Math.PI*Math.random(),t=2*Math.PI*Math.random(),i=Math.random(),n=Math.sqrt(1-i),o=Math.sqrt(i);return this.set(n*Math.sin(e),n*Math.cos(e),o*Math.sin(t),o*Math.cos(t))}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._w===this._w}fromArray(e,t=0){return this._x=e[t],this._y=e[t+1],this._z=e[t+2],this._w=e[t+3],this._onChangeCallback(),this}toArray(e=[],t=0){return e[t]=this._x,e[t+1]=this._y,e[t+2]=this._z,e[t+3]=this._w,e}fromBufferAttribute(e,t){return this._x=e.getX(t),this._y=e.getY(t),this._z=e.getZ(t),this._w=e.getW(t),this._onChangeCallback(),this}toJSON(){return this.toArray()}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._w}};var D=class r{constructor(e=0,t=0,i=0){r.prototype.isVector3=!0,this.x=e,this.y=t,this.z=i}set(e,t,i){return i===void 0&&(i=this.z),this.x=e,this.y=t,this.z=i,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;case 2:this.z=t;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this.z=e.z+t.z,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this.z+=e.z*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this.z=e.z-t.z,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this}multiplyVectors(e,t){return this.x=e.x*t.x,this.y=e.y*t.y,this.z=e.z*t.z,this}applyEuler(e){return this.applyQuaternion(Ol.setFromEuler(e))}applyAxisAngle(e,t){return this.applyQuaternion(Ol.setFromAxisAngle(e,t))}applyMatrix3(e){let t=this.x,i=this.y,n=this.z,o=e.elements;return this.x=o[0]*t+o[3]*i+o[6]*n,this.y=o[1]*t+o[4]*i+o[7]*n,this.z=o[2]*t+o[5]*i+o[8]*n,this}applyNormalMatrix(e){return this.applyMatrix3(e).normalize()}applyMatrix4(e){let t=this.x,i=this.y,n=this.z,o=e.elements,s=1/(o[3]*t+o[7]*i+o[11]*n+o[15]);return this.x=(o[0]*t+o[4]*i+o[8]*n+o[12])*s,this.y=(o[1]*t+o[5]*i+o[9]*n+o[13])*s,this.z=(o[2]*t+o[6]*i+o[10]*n+o[14])*s,this}applyQuaternion(e){let t=this.x,i=this.y,n=this.z,o=e.x,s=e.y,a=e.z,c=e.w,l=2*(s*n-a*i),d=2*(a*t-o*n),u=2*(o*i-s*t);return this.x=t+c*l+s*u-a*d,this.y=i+c*d+a*l-o*u,this.z=n+c*u+o*d-s*l,this}project(e){return this.applyMatrix4(e.matrixWorldInverse).applyMatrix4(e.projectionMatrix)}unproject(e){return this.applyMatrix4(e.projectionMatrixInverse).applyMatrix4(e.matrixWorld)}transformDirection(e){let t=this.x,i=this.y,n=this.z,o=e.elements;return this.x=o[0]*t+o[4]*i+o[8]*n,this.y=o[1]*t+o[5]*i+o[9]*n,this.z=o[2]*t+o[6]*i+o[10]*n,this.normalize()}divide(e){return this.x/=e.x,this.y/=e.y,this.z/=e.z,this}divideScalar(e){return this.multiplyScalar(1/e)}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this}clamp(e,t){return this.x=Math.max(e.x,Math.min(t.x,this.x)),this.y=Math.max(e.y,Math.min(t.y,this.y)),this.z=Math.max(e.z,Math.min(t.z,this.z)),this}clampScalar(e,t){return this.x=Math.max(e,Math.min(t,this.x)),this.y=Math.max(e,Math.min(t,this.y)),this.z=Math.max(e,Math.min(t,this.z)),this}clampLength(e,t){let i=this.length();return this.divideScalar(i||1).multiplyScalar(Math.max(e,Math.min(t,i)))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this.z+=(e.z-this.z)*t,this}lerpVectors(e,t,i){return this.x=e.x+(t.x-e.x)*i,this.y=e.y+(t.y-e.y)*i,this.z=e.z+(t.z-e.z)*i,this}cross(e){return this.crossVectors(this,e)}crossVectors(e,t){let i=e.x,n=e.y,o=e.z,s=t.x,a=t.y,c=t.z;return this.x=n*c-o*a,this.y=o*s-i*c,this.z=i*a-n*s,this}projectOnVector(e){let t=e.lengthSq();if(t===0)return this.set(0,0,0);let i=e.dot(this)/t;return this.copy(e).multiplyScalar(i)}projectOnPlane(e){return xa.copy(this).projectOnVector(e),this.sub(xa)}reflect(e){return this.sub(xa.copy(e).multiplyScalar(2*this.dot(e)))}angleTo(e){let t=Math.sqrt(this.lengthSq()*e.lengthSq());if(t===0)return Math.PI/2;let i=this.dot(e)/t;return Math.acos(Et(i,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){let t=this.x-e.x,i=this.y-e.y,n=this.z-e.z;return t*t+i*i+n*n}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)+Math.abs(this.z-e.z)}setFromSpherical(e){return this.setFromSphericalCoords(e.radius,e.phi,e.theta)}setFromSphericalCoords(e,t,i){let n=Math.sin(t)*e;return this.x=n*Math.sin(i),this.y=Math.cos(t)*e,this.z=n*Math.cos(i),this}setFromCylindrical(e){return this.setFromCylindricalCoords(e.radius,e.theta,e.y)}setFromCylindricalCoords(e,t,i){return this.x=e*Math.sin(t),this.y=i,this.z=e*Math.cos(t),this}setFromMatrixPosition(e){let t=e.elements;return this.x=t[12],this.y=t[13],this.z=t[14],this}setFromMatrixScale(e){let t=this.setFromMatrixColumn(e,0).length(),i=this.setFromMatrixColumn(e,1).length(),n=this.setFromMatrixColumn(e,2).length();return this.x=t,this.y=i,this.z=n,this}setFromMatrixColumn(e,t){return this.fromArray(e.elements,t*4)}setFromMatrix3Column(e,t){return this.fromArray(e.elements,t*3)}setFromEuler(e){return this.x=e._x,this.y=e._y,this.z=e._z,this}setFromColor(e){return this.x=e.r,this.y=e.g,this.z=e.b,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this.z=e[t+2],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e[t+2]=this.z,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this.z=e.getZ(t),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this}randomDirection(){let e=Math.random()*Math.PI*2,t=Math.random()*2-1,i=Math.sqrt(1-t*t);return this.x=i*Math.cos(e),this.y=t,this.z=i*Math.sin(e),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z}},xa=new D,Ol=new Li;var be=class r{constructor(e=0,t=0){r.prototype.isVector2=!0,this.x=e,this.y=t}get width(){return this.x}set width(e){this.x=e}get height(){return this.y}set height(e){this.y=e}set(e,t){return this.x=e,this.y=t,this}setScalar(e){return this.x=e,this.y=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y)}copy(e){return this.x=e.x,this.y=e.y,this}add(e){return this.x+=e.x,this.y+=e.y,this}addScalar(e){return this.x+=e,this.y+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this}subScalar(e){return this.x-=e,this.y-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this}multiply(e){return this.x*=e.x,this.y*=e.y,this}multiplyScalar(e){return this.x*=e,this.y*=e,this}divide(e){return this.x/=e.x,this.y/=e.y,this}divideScalar(e){return this.multiplyScalar(1/e)}applyMatrix3(e){let t=this.x,i=this.y,n=e.elements;return this.x=n[0]*t+n[3]*i+n[6],this.y=n[1]*t+n[4]*i+n[7],this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this}clamp(e,t){return this.x=Math.max(e.x,Math.min(t.x,this.x)),this.y=Math.max(e.y,Math.min(t.y,this.y)),this}clampScalar(e,t){return this.x=Math.max(e,Math.min(t,this.x)),this.y=Math.max(e,Math.min(t,this.y)),this}clampLength(e,t){let i=this.length();return this.divideScalar(i||1).multiplyScalar(Math.max(e,Math.min(t,i)))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this}negate(){return this.x=-this.x,this.y=-this.y,this}dot(e){return this.x*e.x+this.y*e.y}cross(e){return this.x*e.y-this.y*e.x}lengthSq(){return this.x*this.x+this.y*this.y}length(){return Math.sqrt(this.x*this.x+this.y*this.y)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)}normalize(){return this.divideScalar(this.length()||1)}angle(){return Math.atan2(-this.y,-this.x)+Math.PI}angleTo(e){let t=Math.sqrt(this.lengthSq()*e.lengthSq());if(t===0)return Math.PI/2;let i=this.dot(e)/t;return Math.acos(Et(i,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){let t=this.x-e.x,i=this.y-e.y;return t*t+i*i}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this}lerpVectors(e,t,i){return this.x=e.x+(t.x-e.x)*i,this.y=e.y+(t.y-e.y)*i,this}equals(e){return e.x===this.x&&e.y===this.y}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this}rotateAround(e,t){let i=Math.cos(t),n=Math.sin(t),o=this.x-e.x,s=this.y-e.y;return this.x=o*i-s*n+e.x,this.y=o*n+s*i+e.y,this}random(){return this.x=Math.random(),this.y=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y}};var ji=class{constructor(e=new D(1/0,1/0,1/0),t=new D(-1/0,-1/0,-1/0)){this.isBox3=!0,this.min=e,this.max=t}set(e,t){return this.min.copy(e),this.max.copy(t),this}setFromArray(e){this.makeEmpty();for(let t=0,i=e.length;t<i;t+=3)this.expandByPoint(gi.fromArray(e,t));return this}setFromBufferAttribute(e){this.makeEmpty();for(let t=0,i=e.count;t<i;t++)this.expandByPoint(gi.fromBufferAttribute(e,t));return this}setFromPoints(e){this.makeEmpty();for(let t=0,i=e.length;t<i;t++)this.expandByPoint(e[t]);return this}setFromCenterAndSize(e,t){let i=gi.copy(t).multiplyScalar(.5);return this.min.copy(e).sub(i),this.max.copy(e).add(i),this}setFromObject(e,t=!1){return this.makeEmpty(),this.expandByObject(e,t)}clone(){return new this.constructor().copy(this)}copy(e){return this.min.copy(e.min),this.max.copy(e.max),this}makeEmpty(){return this.min.x=this.min.y=this.min.z=1/0,this.max.x=this.max.y=this.max.z=-1/0,this}isEmpty(){return this.max.x<this.min.x||this.max.y<this.min.y||this.max.z<this.min.z}getCenter(e){return this.isEmpty()?e.set(0,0,0):e.addVectors(this.min,this.max).multiplyScalar(.5)}getSize(e){return this.isEmpty()?e.set(0,0,0):e.subVectors(this.max,this.min)}expandByPoint(e){return this.min.min(e),this.max.max(e),this}expandByVector(e){return this.min.sub(e),this.max.add(e),this}expandByScalar(e){return this.min.addScalar(-e),this.max.addScalar(e),this}expandByObject(e,t=!1){e.updateWorldMatrix(!1,!1);let i=e.geometry;if(i!==void 0){let o=i.getAttribute("position");if(t===!0&&o!==void 0&&e.isInstancedMesh!==!0)for(let s=0,a=o.count;s<a;s++)e.isMesh===!0?e.getVertexPosition(s,gi):gi.fromBufferAttribute(o,s),gi.applyMatrix4(e.matrixWorld),this.expandByPoint(gi);else e.boundingBox!==void 0?(e.boundingBox===null&&e.computeBoundingBox(),Go.copy(e.boundingBox)):(i.boundingBox===null&&i.computeBoundingBox(),Go.copy(i.boundingBox)),Go.applyMatrix4(e.matrixWorld),this.union(Go)}let n=e.children;for(let o=0,s=n.length;o<s;o++)this.expandByObject(n[o],t);return this}containsPoint(e){return!(e.x<this.min.x||e.x>this.max.x||e.y<this.min.y||e.y>this.max.y||e.z<this.min.z||e.z>this.max.z)}containsBox(e){return this.min.x<=e.min.x&&e.max.x<=this.max.x&&this.min.y<=e.min.y&&e.max.y<=this.max.y&&this.min.z<=e.min.z&&e.max.z<=this.max.z}getParameter(e,t){return t.set((e.x-this.min.x)/(this.max.x-this.min.x),(e.y-this.min.y)/(this.max.y-this.min.y),(e.z-this.min.z)/(this.max.z-this.min.z))}intersectsBox(e){return!(e.max.x<this.min.x||e.min.x>this.max.x||e.max.y<this.min.y||e.min.y>this.max.y||e.max.z<this.min.z||e.min.z>this.max.z)}intersectsSphere(e){return this.clampPoint(e.center,gi),gi.distanceToSquared(e.center)<=e.radius*e.radius}intersectsPlane(e){let t,i;return e.normal.x>0?(t=e.normal.x*this.min.x,i=e.normal.x*this.max.x):(t=e.normal.x*this.max.x,i=e.normal.x*this.min.x),e.normal.y>0?(t+=e.normal.y*this.min.y,i+=e.normal.y*this.max.y):(t+=e.normal.y*this.max.y,i+=e.normal.y*this.min.y),e.normal.z>0?(t+=e.normal.z*this.min.z,i+=e.normal.z*this.max.z):(t+=e.normal.z*this.max.z,i+=e.normal.z*this.min.z),t<=-e.constant&&i>=-e.constant}intersectsTriangle(e){if(this.isEmpty())return!1;this.getCenter(Zn),Wo.subVectors(this.max,Zn),Yr.subVectors(e.a,Zn),Kr.subVectors(e.b,Zn),Zr.subVectors(e.c,Zn),dr.subVectors(Kr,Yr),ur.subVectors(Zr,Kr),Cr.subVectors(Yr,Zr);let t=[0,-dr.z,dr.y,0,-ur.z,ur.y,0,-Cr.z,Cr.y,dr.z,0,-dr.x,ur.z,0,-ur.x,Cr.z,0,-Cr.x,-dr.y,dr.x,0,-ur.y,ur.x,0,-Cr.y,Cr.x,0];return!va(t,Yr,Kr,Zr,Wo)||(t=[1,0,0,0,1,0,0,0,1],!va(t,Yr,Kr,Zr,Wo))?!1:(Ho.crossVectors(dr,ur),t=[Ho.x,Ho.y,Ho.z],va(t,Yr,Kr,Zr,Wo))}clampPoint(e,t){return t.copy(e).clamp(this.min,this.max)}distanceToPoint(e){return this.clampPoint(e,gi).distanceTo(e)}getBoundingSphere(e){return this.isEmpty()?e.makeEmpty():(this.getCenter(e.center),e.radius=this.getSize(gi).length()*.5),e}intersect(e){return this.min.max(e.min),this.max.min(e.max),this.isEmpty()&&this.makeEmpty(),this}union(e){return this.min.min(e.min),this.max.max(e.max),this}applyMatrix4(e){return this.isEmpty()?this:(Hi[0].set(this.min.x,this.min.y,this.min.z).applyMatrix4(e),Hi[1].set(this.min.x,this.min.y,this.max.z).applyMatrix4(e),Hi[2].set(this.min.x,this.max.y,this.min.z).applyMatrix4(e),Hi[3].set(this.min.x,this.max.y,this.max.z).applyMatrix4(e),Hi[4].set(this.max.x,this.min.y,this.min.z).applyMatrix4(e),Hi[5].set(this.max.x,this.min.y,this.max.z).applyMatrix4(e),Hi[6].set(this.max.x,this.max.y,this.min.z).applyMatrix4(e),Hi[7].set(this.max.x,this.max.y,this.max.z).applyMatrix4(e),this.setFromPoints(Hi),this)}translate(e){return this.min.add(e),this.max.add(e),this}equals(e){return e.min.equals(this.min)&&e.max.equals(this.max)}},Hi=[new D,new D,new D,new D,new D,new D,new D,new D],gi=new D,Go=new ji,Yr=new D,Kr=new D,Zr=new D,dr=new D,ur=new D,Cr=new D,Zn=new D,Wo=new D,Ho=new D,Pr=new D;function va(r,e,t,i,n){for(let o=0,s=r.length-3;o<=s;o+=3){Pr.fromArray(r,o);let a=n.x*Math.abs(Pr.x)+n.y*Math.abs(Pr.y)+n.z*Math.abs(Pr.z),c=e.dot(Pr),l=t.dot(Pr),d=i.dot(Pr);if(Math.max(-Math.max(c,l,d),Math.min(c,l,d))>a)return!1}return!0}var Qt=class{addEventListener(e,t){this._listeners===void 0&&(this._listeners={});let i=this._listeners;i[e]===void 0&&(i[e]=[]),i[e].indexOf(t)===-1&&i[e].push(t)}hasEventListener(e,t){if(this._listeners===void 0)return!1;let i=this._listeners;return i[e]!==void 0&&i[e].indexOf(t)!==-1}removeEventListener(e,t){if(this._listeners===void 0)return;let n=this._listeners[e];if(n!==void 0){let o=n.indexOf(t);o!==-1&&n.splice(o,1)}}dispatchEvent(e){if(this._listeners===void 0)return;let i=this._listeners[e.type];if(i!==void 0){e.target=this;let n=i.slice(0);for(let o=0,s=n.length;o<s;o++)n[o].call(this,e);e.target=null}}};var Nl=0,_a=1,Bl=2;var jo=1,Jn=2,xi=3,ni=0,ut=1,yt=2,di=0,Xi=1,ya=2,Sa=3,ba=4,kl=5,qi=100,zl=101,Vl=102,Gl=103,Wl=104,Hl=200,jl=201,Xl=202,ql=203,Qn=204,eo=205,$l=206,Yl=207,Kl=208,Zl=209,Jl=210,Ql=211,ed=212,td=213,id=214,rd=0,nd=1,od=2,Jr=3,sd=4,ad=5,cd=6,ld=7,Xo=0,dd=1,ud=2,ui=0,hd=1,fd=2,pd=3,md=4,gd=5,xd=6,vd=7;var Ra=300,vi=301,Ui=302,to=303,io=304,hr=306,ro=1e3,Yt=1001,no=1002,nt=1003,_d=1004;var oo=1005;var ht=1006,qo=1007;var _i=1008;var Ut=1009,yd=1010,Sd=1011,$o=1012,Yo=1013,yi=1014,oi=1015,fr=1016,Ko=1017,Zo=1018,Fi=1020,bd=35902,Rd=1021,Md=1022,Mt=1023,wd=1024,Td=1025,Oi=1026,$i=1027,Ed=1028,Jo=1029,Ad=1030,Qo=1031,es=1033,ts=33776,is=33777,rs=33778,ns=33779,Ma=35840,wa=35841,Ta=35842,Ea=35843,Aa=36196,Ca=37492,Pa=37496,Ia=37808,Da=37809,La=37810,Ua=37811,Fa=37812,Oa=37813,Na=37814,Ba=37815,ka=37816,za=37817,Va=37818,Ga=37819,Wa=37820,Ha=37821,os=36492,ja=36494,Xa=36495,Cd=36283,qa=36284,$a=36285,Ya=36286;var Pd=3200,Id=3201,Dd=0,Ld=1,hi="",At="srgb",ei="srgb-linear",Qr="display-p3",Ir="display-p3-linear",en="linear",$e="srgb",tn="rec709",rn="p3";var Dr=7680;var Ka=519,Ud=512,Fd=513,Od=514,ss=515,Nd=516,Bd=517,kd=518,zd=519,Za=35044;var Ja="300 es",ti=2e3,Lr=2001;function as(r){for(let e=r.length-1;e>=0;e-=1)if(r[e]>=65535)return!0;return!1}function Gd(){throw new Error("DOM element creation is unavailable in the PuzzleStudio renderer Worker")}function Wd(){throw new Error("A transferred OffscreenCanvas is required by the PuzzleStudio renderer Worker")}var Vd=new Set;function Hd(r){Vd.has(r)||(Vd.add(r),console.warn(r))}var wt=new D,cs=new be,Kt=class{constructor(e,t,i=!1){if(Array.isArray(e))throw new TypeError("THREE.BufferAttribute: array should be a Typed Array.");this.isBufferAttribute=!0,this.name="",this.array=e,this.itemSize=t,this.count=e!==void 0?e.length/t:0,this.normalized=i,this.usage=Za,this._updateRange={offset:0,count:-1},this.updateRanges=[],this.gpuType=oi,this.version=0}onUploadCallback(){}set needsUpdate(e){e===!0&&this.version++}get updateRange(){return Hd("THREE.BufferAttribute: updateRange() is deprecated and will be removed in r169. Use addUpdateRange() instead."),this._updateRange}setUsage(e){return this.usage=e,this}addUpdateRange(e,t){this.updateRanges.push({start:e,count:t})}clearUpdateRanges(){this.updateRanges.length=0}copy(e){return this.name=e.name,this.array=new e.array.constructor(e.array),this.itemSize=e.itemSize,this.count=e.count,this.normalized=e.normalized,this.usage=e.usage,this.gpuType=e.gpuType,this}copyAt(e,t,i){e*=this.itemSize,i*=t.itemSize;for(let n=0,o=this.itemSize;n<o;n++)this.array[e+n]=t.array[i+n];return this}copyArray(e){return this.array.set(e),this}applyMatrix3(e){if(this.itemSize===2)for(let t=0,i=this.count;t<i;t++)cs.fromBufferAttribute(this,t),cs.applyMatrix3(e),this.setXY(t,cs.x,cs.y);else if(this.itemSize===3)for(let t=0,i=this.count;t<i;t++)wt.fromBufferAttribute(this,t),wt.applyMatrix3(e),this.setXYZ(t,wt.x,wt.y,wt.z);return this}applyMatrix4(e){for(let t=0,i=this.count;t<i;t++)wt.fromBufferAttribute(this,t),wt.applyMatrix4(e),this.setXYZ(t,wt.x,wt.y,wt.z);return this}applyNormalMatrix(e){for(let t=0,i=this.count;t<i;t++)wt.fromBufferAttribute(this,t),wt.applyNormalMatrix(e),this.setXYZ(t,wt.x,wt.y,wt.z);return this}transformDirection(e){for(let t=0,i=this.count;t<i;t++)wt.fromBufferAttribute(this,t),wt.transformDirection(e),this.setXYZ(t,wt.x,wt.y,wt.z);return this}set(e,t=0){return this.array.set(e,t),this}getComponent(e,t){let i=this.array[e*this.itemSize+t];return this.normalized&&(i=$r(i,this.array)),i}setComponent(e,t,i){return this.normalized&&(i=$t(i,this.array)),this.array[e*this.itemSize+t]=i,this}getX(e){let t=this.array[e*this.itemSize];return this.normalized&&(t=$r(t,this.array)),t}setX(e,t){return this.normalized&&(t=$t(t,this.array)),this.array[e*this.itemSize]=t,this}getY(e){let t=this.array[e*this.itemSize+1];return this.normalized&&(t=$r(t,this.array)),t}setY(e,t){return this.normalized&&(t=$t(t,this.array)),this.array[e*this.itemSize+1]=t,this}getZ(e){let t=this.array[e*this.itemSize+2];return this.normalized&&(t=$r(t,this.array)),t}setZ(e,t){return this.normalized&&(t=$t(t,this.array)),this.array[e*this.itemSize+2]=t,this}getW(e){let t=this.array[e*this.itemSize+3];return this.normalized&&(t=$r(t,this.array)),t}setW(e,t){return this.normalized&&(t=$t(t,this.array)),this.array[e*this.itemSize+3]=t,this}setXY(e,t,i){return e*=this.itemSize,this.normalized&&(t=$t(t,this.array),i=$t(i,this.array)),this.array[e+0]=t,this.array[e+1]=i,this}setXYZ(e,t,i,n){return e*=this.itemSize,this.normalized&&(t=$t(t,this.array),i=$t(i,this.array),n=$t(n,this.array)),this.array[e+0]=t,this.array[e+1]=i,this.array[e+2]=n,this}setXYZW(e,t,i,n,o){return e*=this.itemSize,this.normalized&&(t=$t(t,this.array),i=$t(i,this.array),n=$t(n,this.array),o=$t(o,this.array)),this.array[e+0]=t,this.array[e+1]=i,this.array[e+2]=n,this.array[e+3]=o,this}onUpload(e){return this.onUploadCallback=e,this}clone(){return new this.constructor(this.array,this.itemSize).copy(this)}toJSON(){let e={itemSize:this.itemSize,type:this.array.constructor.name,array:Array.from(this.array),normalized:this.normalized};return this.name!==""&&(e.name=this.name),this.usage!==Za&&(e.usage=this.usage),e}};var nn=class extends Kt{constructor(e,t,i){super(new Uint16Array(e),t,i)}};var on=class extends Kt{constructor(e,t,i){super(new Uint32Array(e),t,i)}};var at=class extends Kt{constructor(e,t,i){super(new Float32Array(e),t,i)}};var _g=new ji,so=new D,Qa=new D,pr=class{constructor(e=new D,t=-1){this.isSphere=!0,this.center=e,this.radius=t}set(e,t){return this.center.copy(e),this.radius=t,this}setFromPoints(e,t){let i=this.center;t!==void 0?i.copy(t):_g.setFromPoints(e).getCenter(i);let n=0;for(let o=0,s=e.length;o<s;o++)n=Math.max(n,i.distanceToSquared(e[o]));return this.radius=Math.sqrt(n),this}copy(e){return this.center.copy(e.center),this.radius=e.radius,this}isEmpty(){return this.radius<0}makeEmpty(){return this.center.set(0,0,0),this.radius=-1,this}containsPoint(e){return e.distanceToSquared(this.center)<=this.radius*this.radius}distanceToPoint(e){return e.distanceTo(this.center)-this.radius}intersectsSphere(e){let t=this.radius+e.radius;return e.center.distanceToSquared(this.center)<=t*t}intersectsBox(e){return e.intersectsSphere(this)}intersectsPlane(e){return Math.abs(e.distanceToPoint(this.center))<=this.radius}clampPoint(e,t){let i=this.center.distanceToSquared(e);return t.copy(e),i>this.radius*this.radius&&(t.sub(this.center).normalize(),t.multiplyScalar(this.radius).add(this.center)),t}getBoundingBox(e){return this.isEmpty()?(e.makeEmpty(),e):(e.set(this.center,this.center),e.expandByScalar(this.radius),e)}applyMatrix4(e){return this.center.applyMatrix4(e),this.radius=this.radius*e.getMaxScaleOnAxis(),this}translate(e){return this.center.add(e),this}expandByPoint(e){if(this.isEmpty())return this.center.copy(e),this.radius=0,this;so.subVectors(e,this.center);let t=so.lengthSq();if(t>this.radius*this.radius){let i=Math.sqrt(t),n=(i-this.radius)*.5;this.center.addScaledVector(so,n/i),this.radius+=n}return this}union(e){return e.isEmpty()?this:this.isEmpty()?(this.copy(e),this):(this.center.equals(e.center)===!0?this.radius=Math.max(this.radius,e.radius):(Qa.subVectors(e.center,this.center).setLength(e.radius),this.expandByPoint(so.copy(e.center).add(Qa)),this.expandByPoint(so.copy(e.center).sub(Qa))),this)}equals(e){return e.center.equals(this.center)&&e.radius===this.radius}clone(){return new this.constructor().copy(this)}};var Ee=class r{constructor(e,t,i,n,o,s,a,c,l,d,u,h,p,x,g,m){r.prototype.isMatrix4=!0,this.elements=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],e!==void 0&&this.set(e,t,i,n,o,s,a,c,l,d,u,h,p,x,g,m)}set(e,t,i,n,o,s,a,c,l,d,u,h,p,x,g,m){let f=this.elements;return f[0]=e,f[4]=t,f[8]=i,f[12]=n,f[1]=o,f[5]=s,f[9]=a,f[13]=c,f[2]=l,f[6]=d,f[10]=u,f[14]=h,f[3]=p,f[7]=x,f[11]=g,f[15]=m,this}identity(){return this.set(1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1),this}clone(){return new r().fromArray(this.elements)}copy(e){let t=this.elements,i=e.elements;return t[0]=i[0],t[1]=i[1],t[2]=i[2],t[3]=i[3],t[4]=i[4],t[5]=i[5],t[6]=i[6],t[7]=i[7],t[8]=i[8],t[9]=i[9],t[10]=i[10],t[11]=i[11],t[12]=i[12],t[13]=i[13],t[14]=i[14],t[15]=i[15],this}copyPosition(e){let t=this.elements,i=e.elements;return t[12]=i[12],t[13]=i[13],t[14]=i[14],this}setFromMatrix3(e){let t=e.elements;return this.set(t[0],t[3],t[6],0,t[1],t[4],t[7],0,t[2],t[5],t[8],0,0,0,0,1),this}extractBasis(e,t,i){return e.setFromMatrixColumn(this,0),t.setFromMatrixColumn(this,1),i.setFromMatrixColumn(this,2),this}makeBasis(e,t,i){return this.set(e.x,t.x,i.x,0,e.y,t.y,i.y,0,e.z,t.z,i.z,0,0,0,0,1),this}extractRotation(e){let t=this.elements,i=e.elements,n=1/sn.setFromMatrixColumn(e,0).length(),o=1/sn.setFromMatrixColumn(e,1).length(),s=1/sn.setFromMatrixColumn(e,2).length();return t[0]=i[0]*n,t[1]=i[1]*n,t[2]=i[2]*n,t[3]=0,t[4]=i[4]*o,t[5]=i[5]*o,t[6]=i[6]*o,t[7]=0,t[8]=i[8]*s,t[9]=i[9]*s,t[10]=i[10]*s,t[11]=0,t[12]=0,t[13]=0,t[14]=0,t[15]=1,this}makeRotationFromEuler(e){let t=this.elements,i=e.x,n=e.y,o=e.z,s=Math.cos(i),a=Math.sin(i),c=Math.cos(n),l=Math.sin(n),d=Math.cos(o),u=Math.sin(o);if(e.order==="XYZ"){let h=s*d,p=s*u,x=a*d,g=a*u;t[0]=c*d,t[4]=-c*u,t[8]=l,t[1]=p+x*l,t[5]=h-g*l,t[9]=-a*c,t[2]=g-h*l,t[6]=x+p*l,t[10]=s*c}else if(e.order==="YXZ"){let h=c*d,p=c*u,x=l*d,g=l*u;t[0]=h+g*a,t[4]=x*a-p,t[8]=s*l,t[1]=s*u,t[5]=s*d,t[9]=-a,t[2]=p*a-x,t[6]=g+h*a,t[10]=s*c}else if(e.order==="ZXY"){let h=c*d,p=c*u,x=l*d,g=l*u;t[0]=h-g*a,t[4]=-s*u,t[8]=x+p*a,t[1]=p+x*a,t[5]=s*d,t[9]=g-h*a,t[2]=-s*l,t[6]=a,t[10]=s*c}else if(e.order==="ZYX"){let h=s*d,p=s*u,x=a*d,g=a*u;t[0]=c*d,t[4]=x*l-p,t[8]=h*l+g,t[1]=c*u,t[5]=g*l+h,t[9]=p*l-x,t[2]=-l,t[6]=a*c,t[10]=s*c}else if(e.order==="YZX"){let h=s*c,p=s*l,x=a*c,g=a*l;t[0]=c*d,t[4]=g-h*u,t[8]=x*u+p,t[1]=u,t[5]=s*d,t[9]=-a*d,t[2]=-l*d,t[6]=p*u+x,t[10]=h-g*u}else if(e.order==="XZY"){let h=s*c,p=s*l,x=a*c,g=a*l;t[0]=c*d,t[4]=-u,t[8]=l*d,t[1]=h*u+g,t[5]=s*d,t[9]=p*u-x,t[2]=x*u-p,t[6]=a*d,t[10]=g*u+h}return t[3]=0,t[7]=0,t[11]=0,t[12]=0,t[13]=0,t[14]=0,t[15]=1,this}makeRotationFromQuaternion(e){return this.compose(yg,e,Sg)}lookAt(e,t,i){let n=this.elements;return si.subVectors(e,t),si.lengthSq()===0&&(si.z=1),si.normalize(),mr.crossVectors(i,si),mr.lengthSq()===0&&(Math.abs(i.z)===1?si.x+=1e-4:si.z+=1e-4,si.normalize(),mr.crossVectors(i,si)),mr.normalize(),ls.crossVectors(si,mr),n[0]=mr.x,n[4]=ls.x,n[8]=si.x,n[1]=mr.y,n[5]=ls.y,n[9]=si.y,n[2]=mr.z,n[6]=ls.z,n[10]=si.z,this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,t){let i=e.elements,n=t.elements,o=this.elements,s=i[0],a=i[4],c=i[8],l=i[12],d=i[1],u=i[5],h=i[9],p=i[13],x=i[2],g=i[6],m=i[10],f=i[14],S=i[3],_=i[7],R=i[11],P=i[15],w=n[0],T=n[4],C=n[8],y=n[12],v=n[1],I=n[5],F=n[9],E=n[13],N=n[2],W=n[6],X=n[10],te=n[14],V=n[3],J=n[7],Q=n[11],me=n[15];return o[0]=s*w+a*v+c*N+l*V,o[4]=s*T+a*I+c*W+l*J,o[8]=s*C+a*F+c*X+l*Q,o[12]=s*y+a*E+c*te+l*me,o[1]=d*w+u*v+h*N+p*V,o[5]=d*T+u*I+h*W+p*J,o[9]=d*C+u*F+h*X+p*Q,o[13]=d*y+u*E+h*te+p*me,o[2]=x*w+g*v+m*N+f*V,o[6]=x*T+g*I+m*W+f*J,o[10]=x*C+g*F+m*X+f*Q,o[14]=x*y+g*E+m*te+f*me,o[3]=S*w+_*v+R*N+P*V,o[7]=S*T+_*I+R*W+P*J,o[11]=S*C+_*F+R*X+P*Q,o[15]=S*y+_*E+R*te+P*me,this}multiplyScalar(e){let t=this.elements;return t[0]*=e,t[4]*=e,t[8]*=e,t[12]*=e,t[1]*=e,t[5]*=e,t[9]*=e,t[13]*=e,t[2]*=e,t[6]*=e,t[10]*=e,t[14]*=e,t[3]*=e,t[7]*=e,t[11]*=e,t[15]*=e,this}determinant(){let e=this.elements,t=e[0],i=e[4],n=e[8],o=e[12],s=e[1],a=e[5],c=e[9],l=e[13],d=e[2],u=e[6],h=e[10],p=e[14],x=e[3],g=e[7],m=e[11],f=e[15];return x*(+o*c*u-n*l*u-o*a*h+i*l*h+n*a*p-i*c*p)+g*(+t*c*p-t*l*h+o*s*h-n*s*p+n*l*d-o*c*d)+m*(+t*l*u-t*a*p-o*s*u+i*s*p+o*a*d-i*l*d)+f*(-n*a*d-t*c*u+t*a*h+n*s*u-i*s*h+i*c*d)}transpose(){let e=this.elements,t;return t=e[1],e[1]=e[4],e[4]=t,t=e[2],e[2]=e[8],e[8]=t,t=e[6],e[6]=e[9],e[9]=t,t=e[3],e[3]=e[12],e[12]=t,t=e[7],e[7]=e[13],e[13]=t,t=e[11],e[11]=e[14],e[14]=t,this}setPosition(e,t,i){let n=this.elements;return e.isVector3?(n[12]=e.x,n[13]=e.y,n[14]=e.z):(n[12]=e,n[13]=t,n[14]=i),this}invert(){let e=this.elements,t=e[0],i=e[1],n=e[2],o=e[3],s=e[4],a=e[5],c=e[6],l=e[7],d=e[8],u=e[9],h=e[10],p=e[11],x=e[12],g=e[13],m=e[14],f=e[15],S=u*m*l-g*h*l+g*c*p-a*m*p-u*c*f+a*h*f,_=x*h*l-d*m*l-x*c*p+s*m*p+d*c*f-s*h*f,R=d*g*l-x*u*l+x*a*p-s*g*p-d*a*f+s*u*f,P=x*u*c-d*g*c-x*a*h+s*g*h+d*a*m-s*u*m,w=t*S+i*_+n*R+o*P;if(w===0)return this.set(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0);let T=1/w;return e[0]=S*T,e[1]=(g*h*o-u*m*o-g*n*p+i*m*p+u*n*f-i*h*f)*T,e[2]=(a*m*o-g*c*o+g*n*l-i*m*l-a*n*f+i*c*f)*T,e[3]=(u*c*o-a*h*o-u*n*l+i*h*l+a*n*p-i*c*p)*T,e[4]=_*T,e[5]=(d*m*o-x*h*o+x*n*p-t*m*p-d*n*f+t*h*f)*T,e[6]=(x*c*o-s*m*o-x*n*l+t*m*l+s*n*f-t*c*f)*T,e[7]=(s*h*o-d*c*o+d*n*l-t*h*l-s*n*p+t*c*p)*T,e[8]=R*T,e[9]=(x*u*o-d*g*o-x*i*p+t*g*p+d*i*f-t*u*f)*T,e[10]=(s*g*o-x*a*o+x*i*l-t*g*l-s*i*f+t*a*f)*T,e[11]=(d*a*o-s*u*o-d*i*l+t*u*l+s*i*p-t*a*p)*T,e[12]=P*T,e[13]=(d*g*n-x*u*n+x*i*h-t*g*h-d*i*m+t*u*m)*T,e[14]=(x*a*n-s*g*n-x*i*c+t*g*c+s*i*m-t*a*m)*T,e[15]=(s*u*n-d*a*n+d*i*c-t*u*c-s*i*h+t*a*h)*T,this}scale(e){let t=this.elements,i=e.x,n=e.y,o=e.z;return t[0]*=i,t[4]*=n,t[8]*=o,t[1]*=i,t[5]*=n,t[9]*=o,t[2]*=i,t[6]*=n,t[10]*=o,t[3]*=i,t[7]*=n,t[11]*=o,this}getMaxScaleOnAxis(){let e=this.elements,t=e[0]*e[0]+e[1]*e[1]+e[2]*e[2],i=e[4]*e[4]+e[5]*e[5]+e[6]*e[6],n=e[8]*e[8]+e[9]*e[9]+e[10]*e[10];return Math.sqrt(Math.max(t,i,n))}makeTranslation(e,t,i){return e.isVector3?this.set(1,0,0,e.x,0,1,0,e.y,0,0,1,e.z,0,0,0,1):this.set(1,0,0,e,0,1,0,t,0,0,1,i,0,0,0,1),this}makeRotationX(e){let t=Math.cos(e),i=Math.sin(e);return this.set(1,0,0,0,0,t,-i,0,0,i,t,0,0,0,0,1),this}makeRotationY(e){let t=Math.cos(e),i=Math.sin(e);return this.set(t,0,i,0,0,1,0,0,-i,0,t,0,0,0,0,1),this}makeRotationZ(e){let t=Math.cos(e),i=Math.sin(e);return this.set(t,-i,0,0,i,t,0,0,0,0,1,0,0,0,0,1),this}makeRotationAxis(e,t){let i=Math.cos(t),n=Math.sin(t),o=1-i,s=e.x,a=e.y,c=e.z,l=o*s,d=o*a;return this.set(l*s+i,l*a-n*c,l*c+n*a,0,l*a+n*c,d*a+i,d*c-n*s,0,l*c-n*a,d*c+n*s,o*c*c+i,0,0,0,0,1),this}makeScale(e,t,i){return this.set(e,0,0,0,0,t,0,0,0,0,i,0,0,0,0,1),this}makeShear(e,t,i,n,o,s){return this.set(1,i,o,0,e,1,s,0,t,n,1,0,0,0,0,1),this}compose(e,t,i){let n=this.elements,o=t._x,s=t._y,a=t._z,c=t._w,l=o+o,d=s+s,u=a+a,h=o*l,p=o*d,x=o*u,g=s*d,m=s*u,f=a*u,S=c*l,_=c*d,R=c*u,P=i.x,w=i.y,T=i.z;return n[0]=(1-(g+f))*P,n[1]=(p+R)*P,n[2]=(x-_)*P,n[3]=0,n[4]=(p-R)*w,n[5]=(1-(h+f))*w,n[6]=(m+S)*w,n[7]=0,n[8]=(x+_)*T,n[9]=(m-S)*T,n[10]=(1-(h+g))*T,n[11]=0,n[12]=e.x,n[13]=e.y,n[14]=e.z,n[15]=1,this}decompose(e,t,i){let n=this.elements,o=sn.set(n[0],n[1],n[2]).length(),s=sn.set(n[4],n[5],n[6]).length(),a=sn.set(n[8],n[9],n[10]).length();this.determinant()<0&&(o=-o),e.x=n[12],e.y=n[13],e.z=n[14],Si.copy(this);let l=1/o,d=1/s,u=1/a;return Si.elements[0]*=l,Si.elements[1]*=l,Si.elements[2]*=l,Si.elements[4]*=d,Si.elements[5]*=d,Si.elements[6]*=d,Si.elements[8]*=u,Si.elements[9]*=u,Si.elements[10]*=u,t.setFromRotationMatrix(Si),i.x=o,i.y=s,i.z=a,this}makePerspective(e,t,i,n,o,s,a=ti){let c=this.elements,l=2*o/(t-e),d=2*o/(i-n),u=(t+e)/(t-e),h=(i+n)/(i-n),p,x;if(a===ti)p=-(s+o)/(s-o),x=-2*s*o/(s-o);else if(a===Lr)p=-s/(s-o),x=-s*o/(s-o);else throw new Error("THREE.Matrix4.makePerspective(): Invalid coordinate system: "+a);return c[0]=l,c[4]=0,c[8]=u,c[12]=0,c[1]=0,c[5]=d,c[9]=h,c[13]=0,c[2]=0,c[6]=0,c[10]=p,c[14]=x,c[3]=0,c[7]=0,c[11]=-1,c[15]=0,this}makeOrthographic(e,t,i,n,o,s,a=ti){let c=this.elements,l=1/(t-e),d=1/(i-n),u=1/(s-o),h=(t+e)*l,p=(i+n)*d,x,g;if(a===ti)x=(s+o)*u,g=-2*u;else if(a===Lr)x=o*u,g=-1*u;else throw new Error("THREE.Matrix4.makeOrthographic(): Invalid coordinate system: "+a);return c[0]=2*l,c[4]=0,c[8]=0,c[12]=-h,c[1]=0,c[5]=2*d,c[9]=0,c[13]=-p,c[2]=0,c[6]=0,c[10]=g,c[14]=-x,c[3]=0,c[7]=0,c[11]=0,c[15]=1,this}equals(e){let t=this.elements,i=e.elements;for(let n=0;n<16;n++)if(t[n]!==i[n])return!1;return!0}fromArray(e,t=0){for(let i=0;i<16;i++)this.elements[i]=e[i+t];return this}toArray(e=[],t=0){let i=this.elements;return e[t]=i[0],e[t+1]=i[1],e[t+2]=i[2],e[t+3]=i[3],e[t+4]=i[4],e[t+5]=i[5],e[t+6]=i[6],e[t+7]=i[7],e[t+8]=i[8],e[t+9]=i[9],e[t+10]=i[10],e[t+11]=i[11],e[t+12]=i[12],e[t+13]=i[13],e[t+14]=i[14],e[t+15]=i[15],e}},sn=new D,Si=new Ee,yg=new D(0,0,0),Sg=new D(1,1,1),mr=new D,ls=new D,si=new D;var jd=new Ee,Xd=new Li,ii=class r{constructor(e=0,t=0,i=0,n=r.DEFAULT_ORDER){this.isEuler=!0,this._x=e,this._y=t,this._z=i,this._order=n}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get order(){return this._order}set order(e){this._order=e,this._onChangeCallback()}set(e,t,i,n=this._order){return this._x=e,this._y=t,this._z=i,this._order=n,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._order)}copy(e){return this._x=e._x,this._y=e._y,this._z=e._z,this._order=e._order,this._onChangeCallback(),this}setFromRotationMatrix(e,t=this._order,i=!0){let n=e.elements,o=n[0],s=n[4],a=n[8],c=n[1],l=n[5],d=n[9],u=n[2],h=n[6],p=n[10];switch(t){case"XYZ":this._y=Math.asin(Et(a,-1,1)),Math.abs(a)<.9999999?(this._x=Math.atan2(-d,p),this._z=Math.atan2(-s,o)):(this._x=Math.atan2(h,l),this._z=0);break;case"YXZ":this._x=Math.asin(-Et(d,-1,1)),Math.abs(d)<.9999999?(this._y=Math.atan2(a,p),this._z=Math.atan2(c,l)):(this._y=Math.atan2(-u,o),this._z=0);break;case"ZXY":this._x=Math.asin(Et(h,-1,1)),Math.abs(h)<.9999999?(this._y=Math.atan2(-u,p),this._z=Math.atan2(-s,l)):(this._y=0,this._z=Math.atan2(c,o));break;case"ZYX":this._y=Math.asin(-Et(u,-1,1)),Math.abs(u)<.9999999?(this._x=Math.atan2(h,p),this._z=Math.atan2(c,o)):(this._x=0,this._z=Math.atan2(-s,l));break;case"YZX":this._z=Math.asin(Et(c,-1,1)),Math.abs(c)<.9999999?(this._x=Math.atan2(-d,l),this._y=Math.atan2(-u,o)):(this._x=0,this._y=Math.atan2(a,p));break;case"XZY":this._z=Math.asin(-Et(s,-1,1)),Math.abs(s)<.9999999?(this._x=Math.atan2(h,l),this._y=Math.atan2(a,o)):(this._x=Math.atan2(-d,p),this._y=0);break;default:console.warn("THREE.Euler: .setFromRotationMatrix() encountered an unknown order: "+t)}return this._order=t,i===!0&&this._onChangeCallback(),this}setFromQuaternion(e,t,i){return jd.makeRotationFromQuaternion(e),this.setFromRotationMatrix(jd,t,i)}setFromVector3(e,t=this._order){return this.set(e.x,e.y,e.z,t)}reorder(e){return Xd.setFromEuler(this),this.setFromQuaternion(Xd,e)}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._order===this._order}fromArray(e){return this._x=e[0],this._y=e[1],this._z=e[2],e[3]!==void 0&&(this._order=e[3]),this._onChangeCallback(),this}toArray(e=[],t=0){return e[t]=this._x,e[t+1]=this._y,e[t+2]=this._z,e[t+3]=this._order,e}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._order}};ii.DEFAULT_ORDER="XYZ";var an=class{constructor(){this.mask=1}set(e){this.mask=(1<<e|0)>>>0}enable(e){this.mask|=1<<e|0}enableAll(){this.mask=-1}toggle(e){this.mask^=1<<e|0}disable(e){this.mask&=~(1<<e|0)}disableAll(){this.mask=0}test(e){return(this.mask&e.mask)!==0}isEnabled(e){return(this.mask&(1<<e|0))!==0}};var Se=class r{constructor(e,t,i,n,o,s,a,c,l){r.prototype.isMatrix3=!0,this.elements=[1,0,0,0,1,0,0,0,1],e!==void 0&&this.set(e,t,i,n,o,s,a,c,l)}set(e,t,i,n,o,s,a,c,l){let d=this.elements;return d[0]=e,d[1]=n,d[2]=a,d[3]=t,d[4]=o,d[5]=c,d[6]=i,d[7]=s,d[8]=l,this}identity(){return this.set(1,0,0,0,1,0,0,0,1),this}copy(e){let t=this.elements,i=e.elements;return t[0]=i[0],t[1]=i[1],t[2]=i[2],t[3]=i[3],t[4]=i[4],t[5]=i[5],t[6]=i[6],t[7]=i[7],t[8]=i[8],this}extractBasis(e,t,i){return e.setFromMatrix3Column(this,0),t.setFromMatrix3Column(this,1),i.setFromMatrix3Column(this,2),this}setFromMatrix4(e){let t=e.elements;return this.set(t[0],t[4],t[8],t[1],t[5],t[9],t[2],t[6],t[10]),this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,t){let i=e.elements,n=t.elements,o=this.elements,s=i[0],a=i[3],c=i[6],l=i[1],d=i[4],u=i[7],h=i[2],p=i[5],x=i[8],g=n[0],m=n[3],f=n[6],S=n[1],_=n[4],R=n[7],P=n[2],w=n[5],T=n[8];return o[0]=s*g+a*S+c*P,o[3]=s*m+a*_+c*w,o[6]=s*f+a*R+c*T,o[1]=l*g+d*S+u*P,o[4]=l*m+d*_+u*w,o[7]=l*f+d*R+u*T,o[2]=h*g+p*S+x*P,o[5]=h*m+p*_+x*w,o[8]=h*f+p*R+x*T,this}multiplyScalar(e){let t=this.elements;return t[0]*=e,t[3]*=e,t[6]*=e,t[1]*=e,t[4]*=e,t[7]*=e,t[2]*=e,t[5]*=e,t[8]*=e,this}determinant(){let e=this.elements,t=e[0],i=e[1],n=e[2],o=e[3],s=e[4],a=e[5],c=e[6],l=e[7],d=e[8];return t*s*d-t*a*l-i*o*d+i*a*c+n*o*l-n*s*c}invert(){let e=this.elements,t=e[0],i=e[1],n=e[2],o=e[3],s=e[4],a=e[5],c=e[6],l=e[7],d=e[8],u=d*s-a*l,h=a*c-d*o,p=l*o-s*c,x=t*u+i*h+n*p;if(x===0)return this.set(0,0,0,0,0,0,0,0,0);let g=1/x;return e[0]=u*g,e[1]=(n*l-d*i)*g,e[2]=(a*i-n*s)*g,e[3]=h*g,e[4]=(d*t-n*c)*g,e[5]=(n*o-a*t)*g,e[6]=p*g,e[7]=(i*c-l*t)*g,e[8]=(s*t-i*o)*g,this}transpose(){let e,t=this.elements;return e=t[1],t[1]=t[3],t[3]=e,e=t[2],t[2]=t[6],t[6]=e,e=t[5],t[5]=t[7],t[7]=e,this}getNormalMatrix(e){return this.setFromMatrix4(e).invert().transpose()}transposeIntoArray(e){let t=this.elements;return e[0]=t[0],e[1]=t[3],e[2]=t[6],e[3]=t[1],e[4]=t[4],e[5]=t[7],e[6]=t[2],e[7]=t[5],e[8]=t[8],this}setUvTransform(e,t,i,n,o,s,a){let c=Math.cos(o),l=Math.sin(o);return this.set(i*c,i*l,-i*(c*s+l*a)+s+e,-n*l,n*c,-n*(-l*s+c*a)+a+t,0,0,1),this}scale(e,t){return this.premultiply(ec.makeScale(e,t)),this}rotate(e){return this.premultiply(ec.makeRotation(-e)),this}translate(e,t){return this.premultiply(ec.makeTranslation(e,t)),this}makeTranslation(e,t){return e.isVector2?this.set(1,0,e.x,0,1,e.y,0,0,1):this.set(1,0,e,0,1,t,0,0,1),this}makeRotation(e){let t=Math.cos(e),i=Math.sin(e);return this.set(t,-i,0,i,t,0,0,0,1),this}makeScale(e,t){return this.set(e,0,0,0,t,0,0,0,1),this}equals(e){let t=this.elements,i=e.elements;for(let n=0;n<9;n++)if(t[n]!==i[n])return!1;return!0}fromArray(e,t=0){for(let i=0;i<9;i++)this.elements[i]=e[i+t];return this}toArray(e=[],t=0){let i=this.elements;return e[t]=i[0],e[t+1]=i[1],e[t+2]=i[2],e[t+3]=i[3],e[t+4]=i[4],e[t+5]=i[5],e[t+6]=i[6],e[t+7]=i[7],e[t+8]=i[8],e}clone(){return new this.constructor().fromArray(this.elements)}},ec=new Se;var bg=0,qd=new D,cn=new Li,Yi=new Ee,ds=new D,ao=new D,Rg=new D,Mg=new Li,$d=new D(1,0,0),Yd=new D(0,1,0),Kd=new D(0,0,1),Zd={type:"added"},wg={type:"removed"},ln={type:"childadded",child:null},tc={type:"childremoved",child:null},vt=class r extends Qt{constructor(){super(),this.isObject3D=!0,Object.defineProperty(this,"id",{value:bg++}),this.uuid=Di(),this.name="",this.type="Object3D",this.parent=null,this.children=[],this.up=r.DEFAULT_UP.clone();let e=new D,t=new ii,i=new Li,n=new D(1,1,1);function o(){i.setFromEuler(t,!1)}function s(){t.setFromQuaternion(i,void 0,!1)}t._onChange(o),i._onChange(s),Object.defineProperties(this,{position:{configurable:!0,enumerable:!0,value:e},rotation:{configurable:!0,enumerable:!0,value:t},quaternion:{configurable:!0,enumerable:!0,value:i},scale:{configurable:!0,enumerable:!0,value:n},modelViewMatrix:{value:new Ee},normalMatrix:{value:new Se}}),this.matrix=new Ee,this.matrixWorld=new Ee,this.matrixAutoUpdate=r.DEFAULT_MATRIX_AUTO_UPDATE,this.matrixWorldAutoUpdate=r.DEFAULT_MATRIX_WORLD_AUTO_UPDATE,this.matrixWorldNeedsUpdate=!1,this.layers=new an,this.visible=!0,this.castShadow=!1,this.receiveShadow=!1,this.frustumCulled=!0,this.renderOrder=0,this.animations=[],this.userData={}}onBeforeShadow(){}onAfterShadow(){}onBeforeRender(){}onAfterRender(){}applyMatrix4(e){this.matrixAutoUpdate&&this.updateMatrix(),this.matrix.premultiply(e),this.matrix.decompose(this.position,this.quaternion,this.scale)}applyQuaternion(e){return this.quaternion.premultiply(e),this}setRotationFromAxisAngle(e,t){this.quaternion.setFromAxisAngle(e,t)}setRotationFromEuler(e){this.quaternion.setFromEuler(e,!0)}setRotationFromMatrix(e){this.quaternion.setFromRotationMatrix(e)}setRotationFromQuaternion(e){this.quaternion.copy(e)}rotateOnAxis(e,t){return cn.setFromAxisAngle(e,t),this.quaternion.multiply(cn),this}rotateOnWorldAxis(e,t){return cn.setFromAxisAngle(e,t),this.quaternion.premultiply(cn),this}rotateX(e){return this.rotateOnAxis($d,e)}rotateY(e){return this.rotateOnAxis(Yd,e)}rotateZ(e){return this.rotateOnAxis(Kd,e)}translateOnAxis(e,t){return qd.copy(e).applyQuaternion(this.quaternion),this.position.add(qd.multiplyScalar(t)),this}translateX(e){return this.translateOnAxis($d,e)}translateY(e){return this.translateOnAxis(Yd,e)}translateZ(e){return this.translateOnAxis(Kd,e)}localToWorld(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4(this.matrixWorld)}worldToLocal(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4(Yi.copy(this.matrixWorld).invert())}lookAt(e,t,i){e.isVector3?ds.copy(e):ds.set(e,t,i);let n=this.parent;this.updateWorldMatrix(!0,!1),ao.setFromMatrixPosition(this.matrixWorld),this.isCamera||this.isLight?Yi.lookAt(ao,ds,this.up):Yi.lookAt(ds,ao,this.up),this.quaternion.setFromRotationMatrix(Yi),n&&(Yi.extractRotation(n.matrixWorld),cn.setFromRotationMatrix(Yi),this.quaternion.premultiply(cn.invert()))}add(e){if(arguments.length>1){for(let t=0;t<arguments.length;t++)this.add(arguments[t]);return this}return e===this?(console.error("THREE.Object3D.add: object can't be added as a child of itself.",e),this):(e&&e.isObject3D?(e.removeFromParent(),e.parent=this,this.children.push(e),e.dispatchEvent(Zd),ln.child=e,this.dispatchEvent(ln),ln.child=null):console.error("THREE.Object3D.add: object not an instance of THREE.Object3D.",e),this)}remove(e){if(arguments.length>1){for(let i=0;i<arguments.length;i++)this.remove(arguments[i]);return this}let t=this.children.indexOf(e);return t!==-1&&(e.parent=null,this.children.splice(t,1),e.dispatchEvent(wg),tc.child=e,this.dispatchEvent(tc),tc.child=null),this}removeFromParent(){let e=this.parent;return e!==null&&e.remove(this),this}clear(){return this.remove(...this.children)}attach(e){return this.updateWorldMatrix(!0,!1),Yi.copy(this.matrixWorld).invert(),e.parent!==null&&(e.parent.updateWorldMatrix(!0,!1),Yi.multiply(e.parent.matrixWorld)),e.applyMatrix4(Yi),e.removeFromParent(),e.parent=this,this.children.push(e),e.updateWorldMatrix(!1,!0),e.dispatchEvent(Zd),ln.child=e,this.dispatchEvent(ln),ln.child=null,this}getObjectById(e){return this.getObjectByProperty("id",e)}getObjectByName(e){return this.getObjectByProperty("name",e)}getObjectByProperty(e,t){if(this[e]===t)return this;for(let i=0,n=this.children.length;i<n;i++){let s=this.children[i].getObjectByProperty(e,t);if(s!==void 0)return s}}getObjectsByProperty(e,t,i=[]){this[e]===t&&i.push(this);let n=this.children;for(let o=0,s=n.length;o<s;o++)n[o].getObjectsByProperty(e,t,i);return i}getWorldPosition(e){return this.updateWorldMatrix(!0,!1),e.setFromMatrixPosition(this.matrixWorld)}getWorldQuaternion(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(ao,e,Rg),e}getWorldScale(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(ao,Mg,e),e}getWorldDirection(e){this.updateWorldMatrix(!0,!1);let t=this.matrixWorld.elements;return e.set(t[8],t[9],t[10]).normalize()}raycast(){}traverse(e){e(this);let t=this.children;for(let i=0,n=t.length;i<n;i++)t[i].traverse(e)}traverseVisible(e){if(this.visible===!1)return;e(this);let t=this.children;for(let i=0,n=t.length;i<n;i++)t[i].traverseVisible(e)}traverseAncestors(e){let t=this.parent;t!==null&&(e(t),t.traverseAncestors(e))}updateMatrix(){this.matrix.compose(this.position,this.quaternion,this.scale),this.matrixWorldNeedsUpdate=!0}updateMatrixWorld(e){this.matrixAutoUpdate&&this.updateMatrix(),(this.matrixWorldNeedsUpdate||e)&&(this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix),this.matrixWorldNeedsUpdate=!1,e=!0);let t=this.children;for(let i=0,n=t.length;i<n;i++){let o=t[i];(o.matrixWorldAutoUpdate===!0||e===!0)&&o.updateMatrixWorld(e)}}updateWorldMatrix(e,t){let i=this.parent;if(e===!0&&i!==null&&i.matrixWorldAutoUpdate===!0&&i.updateWorldMatrix(!0,!1),this.matrixAutoUpdate&&this.updateMatrix(),this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix),t===!0){let n=this.children;for(let o=0,s=n.length;o<s;o++){let a=n[o];a.matrixWorldAutoUpdate===!0&&a.updateWorldMatrix(!1,!0)}}}toJSON(e){let t=e===void 0||typeof e=="string",i={};t&&(e={geometries:{},materials:{},textures:{},images:{},shapes:{},skeletons:{},animations:{},nodes:{}},i.metadata={version:4.6,type:"Object",generator:"Object3D.toJSON"});let n={};n.uuid=this.uuid,n.type=this.type,this.name!==""&&(n.name=this.name),this.castShadow===!0&&(n.castShadow=!0),this.receiveShadow===!0&&(n.receiveShadow=!0),this.visible===!1&&(n.visible=!1),this.frustumCulled===!1&&(n.frustumCulled=!1),this.renderOrder!==0&&(n.renderOrder=this.renderOrder),Object.keys(this.userData).length>0&&(n.userData=this.userData),n.layers=this.layers.mask,n.matrix=this.matrix.toArray(),n.up=this.up.toArray(),this.matrixAutoUpdate===!1&&(n.matrixAutoUpdate=!1),this.isInstancedMesh&&(n.type="InstancedMesh",n.count=this.count,n.instanceMatrix=this.instanceMatrix.toJSON(),this.instanceColor!==null&&(n.instanceColor=this.instanceColor.toJSON())),this.isBatchedMesh&&(n.type="BatchedMesh",n.perObjectFrustumCulled=this.perObjectFrustumCulled,n.sortObjects=this.sortObjects,n.drawRanges=this._drawRanges,n.reservedRanges=this._reservedRanges,n.visibility=this._visibility,n.active=this._active,n.bounds=this._bounds.map(a=>({boxInitialized:a.boxInitialized,boxMin:a.box.min.toArray(),boxMax:a.box.max.toArray(),sphereInitialized:a.sphereInitialized,sphereRadius:a.sphere.radius,sphereCenter:a.sphere.center.toArray()})),n.maxGeometryCount=this._maxGeometryCount,n.maxVertexCount=this._maxVertexCount,n.maxIndexCount=this._maxIndexCount,n.geometryInitialized=this._geometryInitialized,n.geometryCount=this._geometryCount,n.matricesTexture=this._matricesTexture.toJSON(e),this.boundingSphere!==null&&(n.boundingSphere={center:n.boundingSphere.center.toArray(),radius:n.boundingSphere.radius}),this.boundingBox!==null&&(n.boundingBox={min:n.boundingBox.min.toArray(),max:n.boundingBox.max.toArray()}));function o(a,c){return a[c.uuid]===void 0&&(a[c.uuid]=c.toJSON(e)),c.uuid}if(this.isScene)this.background&&(this.background.isColor?n.background=this.background.toJSON():this.background.isTexture&&(n.background=this.background.toJSON(e).uuid)),this.environment&&this.environment.isTexture&&this.environment.isRenderTargetTexture!==!0&&(n.environment=this.environment.toJSON(e).uuid);else if(this.isMesh||this.isLine||this.isPoints){n.geometry=o(e.geometries,this.geometry);let a=this.geometry.parameters;if(a!==void 0&&a.shapes!==void 0){let c=a.shapes;if(Array.isArray(c))for(let l=0,d=c.length;l<d;l++){let u=c[l];o(e.shapes,u)}else o(e.shapes,c)}}if(this.isSkinnedMesh&&(n.bindMode=this.bindMode,n.bindMatrix=this.bindMatrix.toArray(),this.skeleton!==void 0&&(o(e.skeletons,this.skeleton),n.skeleton=this.skeleton.uuid)),this.material!==void 0)if(Array.isArray(this.material)){let a=[];for(let c=0,l=this.material.length;c<l;c++)a.push(o(e.materials,this.material[c]));n.material=a}else n.material=o(e.materials,this.material);if(this.children.length>0){n.children=[];for(let a=0;a<this.children.length;a++)n.children.push(this.children[a].toJSON(e).object)}if(this.animations.length>0){n.animations=[];for(let a=0;a<this.animations.length;a++){let c=this.animations[a];n.animations.push(o(e.animations,c))}}if(t){let a=s(e.geometries),c=s(e.materials),l=s(e.textures),d=s(e.images),u=s(e.shapes),h=s(e.skeletons),p=s(e.animations),x=s(e.nodes);a.length>0&&(i.geometries=a),c.length>0&&(i.materials=c),l.length>0&&(i.textures=l),d.length>0&&(i.images=d),u.length>0&&(i.shapes=u),h.length>0&&(i.skeletons=h),p.length>0&&(i.animations=p),x.length>0&&(i.nodes=x)}return i.object=n,i;function s(a){let c=[];for(let l in a){let d=a[l];delete d.metadata,c.push(d)}return c}}clone(e){return new this.constructor().copy(this,e)}copy(e,t=!0){if(this.name=e.name,this.up.copy(e.up),this.position.copy(e.position),this.rotation.order=e.rotation.order,this.quaternion.copy(e.quaternion),this.scale.copy(e.scale),this.matrix.copy(e.matrix),this.matrixWorld.copy(e.matrixWorld),this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrixWorldAutoUpdate=e.matrixWorldAutoUpdate,this.matrixWorldNeedsUpdate=e.matrixWorldNeedsUpdate,this.layers.mask=e.layers.mask,this.visible=e.visible,this.castShadow=e.castShadow,this.receiveShadow=e.receiveShadow,this.frustumCulled=e.frustumCulled,this.renderOrder=e.renderOrder,this.animations=e.animations.slice(),this.userData=JSON.parse(JSON.stringify(e.userData)),t===!0)for(let i=0;i<e.children.length;i++){let n=e.children[i];this.add(n.clone())}return this}};vt.DEFAULT_UP=new D(0,1,0);vt.DEFAULT_MATRIX_AUTO_UPDATE=!0;vt.DEFAULT_MATRIX_WORLD_AUTO_UPDATE=!0;var Tg=0,fi=new Ee,ic=new vt,dn=new D,ai=new ji,co=new ji,Ft=new D,Tt=class r extends Qt{constructor(){super(),this.isBufferGeometry=!0,Object.defineProperty(this,"id",{value:Tg++}),this.uuid=Di(),this.name="",this.type="BufferGeometry",this.index=null,this.attributes={},this.morphAttributes={},this.morphTargetsRelative=!1,this.groups=[],this.boundingBox=null,this.boundingSphere=null,this.drawRange={start:0,count:1/0},this.userData={}}getIndex(){return this.index}setIndex(e){return Array.isArray(e)?this.index=new(as(e)?on:nn)(e,1):this.index=e,this}getAttribute(e){return this.attributes[e]}setAttribute(e,t){return this.attributes[e]=t,this}deleteAttribute(e){return delete this.attributes[e],this}hasAttribute(e){return this.attributes[e]!==void 0}addGroup(e,t,i=0){this.groups.push({start:e,count:t,materialIndex:i})}clearGroups(){this.groups=[]}setDrawRange(e,t){this.drawRange.start=e,this.drawRange.count=t}applyMatrix4(e){let t=this.attributes.position;t!==void 0&&(t.applyMatrix4(e),t.needsUpdate=!0);let i=this.attributes.normal;if(i!==void 0){let o=new Se().getNormalMatrix(e);i.applyNormalMatrix(o),i.needsUpdate=!0}let n=this.attributes.tangent;return n!==void 0&&(n.transformDirection(e),n.needsUpdate=!0),this.boundingBox!==null&&this.computeBoundingBox(),this.boundingSphere!==null&&this.computeBoundingSphere(),this}applyQuaternion(e){return fi.makeRotationFromQuaternion(e),this.applyMatrix4(fi),this}rotateX(e){return fi.makeRotationX(e),this.applyMatrix4(fi),this}rotateY(e){return fi.makeRotationY(e),this.applyMatrix4(fi),this}rotateZ(e){return fi.makeRotationZ(e),this.applyMatrix4(fi),this}translate(e,t,i){return fi.makeTranslation(e,t,i),this.applyMatrix4(fi),this}scale(e,t,i){return fi.makeScale(e,t,i),this.applyMatrix4(fi),this}lookAt(e){return ic.lookAt(e),ic.updateMatrix(),this.applyMatrix4(ic.matrix),this}center(){return this.computeBoundingBox(),this.boundingBox.getCenter(dn).negate(),this.translate(dn.x,dn.y,dn.z),this}setFromPoints(e){let t=[];for(let i=0,n=e.length;i<n;i++){let o=e[i];t.push(o.x,o.y,o.z||0)}return this.setAttribute("position",new at(t,3)),this}computeBoundingBox(){this.boundingBox===null&&(this.boundingBox=new ji);let e=this.attributes.position,t=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){console.error("THREE.BufferGeometry.computeBoundingBox(): GLBufferAttribute requires a manual bounding box.",this),this.boundingBox.set(new D(-1/0,-1/0,-1/0),new D(1/0,1/0,1/0));return}if(e!==void 0){if(this.boundingBox.setFromBufferAttribute(e),t)for(let i=0,n=t.length;i<n;i++){let o=t[i];ai.setFromBufferAttribute(o),this.morphTargetsRelative?(Ft.addVectors(this.boundingBox.min,ai.min),this.boundingBox.expandByPoint(Ft),Ft.addVectors(this.boundingBox.max,ai.max),this.boundingBox.expandByPoint(Ft)):(this.boundingBox.expandByPoint(ai.min),this.boundingBox.expandByPoint(ai.max))}}else this.boundingBox.makeEmpty();(isNaN(this.boundingBox.min.x)||isNaN(this.boundingBox.min.y)||isNaN(this.boundingBox.min.z))&&console.error('THREE.BufferGeometry.computeBoundingBox(): Computed min/max have NaN values. The "position" attribute is likely to have NaN values.',this)}computeBoundingSphere(){this.boundingSphere===null&&(this.boundingSphere=new pr);let e=this.attributes.position,t=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){console.error("THREE.BufferGeometry.computeBoundingSphere(): GLBufferAttribute requires a manual bounding sphere.",this),this.boundingSphere.set(new D,1/0);return}if(e){let i=this.boundingSphere.center;if(ai.setFromBufferAttribute(e),t)for(let o=0,s=t.length;o<s;o++){let a=t[o];co.setFromBufferAttribute(a),this.morphTargetsRelative?(Ft.addVectors(ai.min,co.min),ai.expandByPoint(Ft),Ft.addVectors(ai.max,co.max),ai.expandByPoint(Ft)):(ai.expandByPoint(co.min),ai.expandByPoint(co.max))}ai.getCenter(i);let n=0;for(let o=0,s=e.count;o<s;o++)Ft.fromBufferAttribute(e,o),n=Math.max(n,i.distanceToSquared(Ft));if(t)for(let o=0,s=t.length;o<s;o++){let a=t[o],c=this.morphTargetsRelative;for(let l=0,d=a.count;l<d;l++)Ft.fromBufferAttribute(a,l),c&&(dn.fromBufferAttribute(e,l),Ft.add(dn)),n=Math.max(n,i.distanceToSquared(Ft))}this.boundingSphere.radius=Math.sqrt(n),isNaN(this.boundingSphere.radius)&&console.error('THREE.BufferGeometry.computeBoundingSphere(): Computed radius is NaN. The "position" attribute is likely to have NaN values.',this)}}computeTangents(){let e=this.index,t=this.attributes;if(e===null||t.position===void 0||t.normal===void 0||t.uv===void 0){console.error("THREE.BufferGeometry: .computeTangents() failed. Missing required attributes (index, position, normal or uv)");return}let i=t.position,n=t.normal,o=t.uv;this.hasAttribute("tangent")===!1&&this.setAttribute("tangent",new Kt(new Float32Array(4*i.count),4));let s=this.getAttribute("tangent"),a=[],c=[];for(let C=0;C<i.count;C++)a[C]=new D,c[C]=new D;let l=new D,d=new D,u=new D,h=new be,p=new be,x=new be,g=new D,m=new D;function f(C,y,v){l.fromBufferAttribute(i,C),d.fromBufferAttribute(i,y),u.fromBufferAttribute(i,v),h.fromBufferAttribute(o,C),p.fromBufferAttribute(o,y),x.fromBufferAttribute(o,v),d.sub(l),u.sub(l),p.sub(h),x.sub(h);let I=1/(p.x*x.y-x.x*p.y);isFinite(I)&&(g.copy(d).multiplyScalar(x.y).addScaledVector(u,-p.y).multiplyScalar(I),m.copy(u).multiplyScalar(p.x).addScaledVector(d,-x.x).multiplyScalar(I),a[C].add(g),a[y].add(g),a[v].add(g),c[C].add(m),c[y].add(m),c[v].add(m))}let S=this.groups;S.length===0&&(S=[{start:0,count:e.count}]);for(let C=0,y=S.length;C<y;++C){let v=S[C],I=v.start,F=v.count;for(let E=I,N=I+F;E<N;E+=3)f(e.getX(E+0),e.getX(E+1),e.getX(E+2))}let _=new D,R=new D,P=new D,w=new D;function T(C){P.fromBufferAttribute(n,C),w.copy(P);let y=a[C];_.copy(y),_.sub(P.multiplyScalar(P.dot(y))).normalize(),R.crossVectors(w,y);let I=R.dot(c[C])<0?-1:1;s.setXYZW(C,_.x,_.y,_.z,I)}for(let C=0,y=S.length;C<y;++C){let v=S[C],I=v.start,F=v.count;for(let E=I,N=I+F;E<N;E+=3)T(e.getX(E+0)),T(e.getX(E+1)),T(e.getX(E+2))}}computeVertexNormals(){let e=this.index,t=this.getAttribute("position");if(t!==void 0){let i=this.getAttribute("normal");if(i===void 0)i=new Kt(new Float32Array(t.count*3),3),this.setAttribute("normal",i);else for(let h=0,p=i.count;h<p;h++)i.setXYZ(h,0,0,0);let n=new D,o=new D,s=new D,a=new D,c=new D,l=new D,d=new D,u=new D;if(e)for(let h=0,p=e.count;h<p;h+=3){let x=e.getX(h+0),g=e.getX(h+1),m=e.getX(h+2);n.fromBufferAttribute(t,x),o.fromBufferAttribute(t,g),s.fromBufferAttribute(t,m),d.subVectors(s,o),u.subVectors(n,o),d.cross(u),a.fromBufferAttribute(i,x),c.fromBufferAttribute(i,g),l.fromBufferAttribute(i,m),a.add(d),c.add(d),l.add(d),i.setXYZ(x,a.x,a.y,a.z),i.setXYZ(g,c.x,c.y,c.z),i.setXYZ(m,l.x,l.y,l.z)}else for(let h=0,p=t.count;h<p;h+=3)n.fromBufferAttribute(t,h+0),o.fromBufferAttribute(t,h+1),s.fromBufferAttribute(t,h+2),d.subVectors(s,o),u.subVectors(n,o),d.cross(u),i.setXYZ(h+0,d.x,d.y,d.z),i.setXYZ(h+1,d.x,d.y,d.z),i.setXYZ(h+2,d.x,d.y,d.z);this.normalizeNormals(),i.needsUpdate=!0}}normalizeNormals(){let e=this.attributes.normal;for(let t=0,i=e.count;t<i;t++)Ft.fromBufferAttribute(e,t),Ft.normalize(),e.setXYZ(t,Ft.x,Ft.y,Ft.z)}toNonIndexed(){function e(a,c){let l=a.array,d=a.itemSize,u=a.normalized,h=new l.constructor(c.length*d),p=0,x=0;for(let g=0,m=c.length;g<m;g++){a.isInterleavedBufferAttribute?p=c[g]*a.data.stride+a.offset:p=c[g]*d;for(let f=0;f<d;f++)h[x++]=l[p++]}return new Kt(h,d,u)}if(this.index===null)return console.warn("THREE.BufferGeometry.toNonIndexed(): BufferGeometry is already non-indexed."),this;let t=new r,i=this.index.array,n=this.attributes;for(let a in n){let c=n[a],l=e(c,i);t.setAttribute(a,l)}let o=this.morphAttributes;for(let a in o){let c=[],l=o[a];for(let d=0,u=l.length;d<u;d++){let h=l[d],p=e(h,i);c.push(p)}t.morphAttributes[a]=c}t.morphTargetsRelative=this.morphTargetsRelative;let s=this.groups;for(let a=0,c=s.length;a<c;a++){let l=s[a];t.addGroup(l.start,l.count,l.materialIndex)}return t}toJSON(){let e={metadata:{version:4.6,type:"BufferGeometry",generator:"BufferGeometry.toJSON"}};if(e.uuid=this.uuid,e.type=this.type,this.name!==""&&(e.name=this.name),Object.keys(this.userData).length>0&&(e.userData=this.userData),this.parameters!==void 0){let c=this.parameters;for(let l in c)c[l]!==void 0&&(e[l]=c[l]);return e}e.data={attributes:{}};let t=this.index;t!==null&&(e.data.index={type:t.array.constructor.name,array:Array.prototype.slice.call(t.array)});let i=this.attributes;for(let c in i){let l=i[c];e.data.attributes[c]=l.toJSON(e.data)}let n={},o=!1;for(let c in this.morphAttributes){let l=this.morphAttributes[c],d=[];for(let u=0,h=l.length;u<h;u++){let p=l[u];d.push(p.toJSON(e.data))}d.length>0&&(n[c]=d,o=!0)}o&&(e.data.morphAttributes=n,e.data.morphTargetsRelative=this.morphTargetsRelative);let s=this.groups;s.length>0&&(e.data.groups=JSON.parse(JSON.stringify(s)));let a=this.boundingSphere;return a!==null&&(e.data.boundingSphere={center:a.center.toArray(),radius:a.radius}),e}clone(){return new this.constructor().copy(this)}copy(e){this.index=null,this.attributes={},this.morphAttributes={},this.groups=[],this.boundingBox=null,this.boundingSphere=null;let t={};this.name=e.name;let i=e.index;i!==null&&this.setIndex(i.clone(t));let n=e.attributes;for(let l in n){let d=n[l];this.setAttribute(l,d.clone(t))}let o=e.morphAttributes;for(let l in o){let d=[],u=o[l];for(let h=0,p=u.length;h<p;h++)d.push(u[h].clone(t));this.morphAttributes[l]=d}this.morphTargetsRelative=e.morphTargetsRelative;let s=e.groups;for(let l=0,d=s.length;l<d;l++){let u=s[l];this.addGroup(u.start,u.count,u.materialIndex)}let a=e.boundingBox;a!==null&&(this.boundingBox=a.clone());let c=e.boundingSphere;return c!==null&&(this.boundingSphere=c.clone()),this.drawRange.start=e.drawRange.start,this.drawRange.count=e.drawRange.count,this.userData=e.userData,this}dispose(){this.dispatchEvent({type:"dispose"})}};var Ni=class extends vt{constructor(){super(),this.isCamera=!0,this.type="Camera",this.matrixWorldInverse=new Ee,this.projectionMatrix=new Ee,this.projectionMatrixInverse=new Ee,this.coordinateSystem=ti}copy(e,t){return super.copy(e,t),this.matrixWorldInverse.copy(e.matrixWorldInverse),this.projectionMatrix.copy(e.projectionMatrix),this.projectionMatrixInverse.copy(e.projectionMatrixInverse),this.coordinateSystem=e.coordinateSystem,this}getWorldDirection(e){return super.getWorldDirection(e).negate()}updateMatrixWorld(e){super.updateMatrixWorld(e),this.matrixWorldInverse.copy(this.matrixWorld).invert()}updateWorldMatrix(e,t){super.updateWorldMatrix(e,t),this.matrixWorldInverse.copy(this.matrixWorld).invert()}clone(){return new this.constructor().copy(this)}};var un=class extends Ni{constructor(e=-1,t=1,i=1,n=-1,o=.1,s=2e3){super(),this.isOrthographicCamera=!0,this.type="OrthographicCamera",this.zoom=1,this.view=null,this.left=e,this.right=t,this.top=i,this.bottom=n,this.near=o,this.far=s,this.updateProjectionMatrix()}copy(e,t){return super.copy(e,t),this.left=e.left,this.right=e.right,this.top=e.top,this.bottom=e.bottom,this.near=e.near,this.far=e.far,this.zoom=e.zoom,this.view=e.view===null?null:Object.assign({},e.view),this}setViewOffset(e,t,i,n,o,s){this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=t,this.view.offsetX=i,this.view.offsetY=n,this.view.width=o,this.view.height=s,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){let e=(this.right-this.left)/(2*this.zoom),t=(this.top-this.bottom)/(2*this.zoom),i=(this.right+this.left)/2,n=(this.top+this.bottom)/2,o=i-e,s=i+e,a=n+t,c=n-t;if(this.view!==null&&this.view.enabled){let l=(this.right-this.left)/this.view.fullWidth/this.zoom,d=(this.top-this.bottom)/this.view.fullHeight/this.zoom;o+=l*this.view.offsetX,s=o+l*this.view.width,a-=d*this.view.offsetY,c=a-d*this.view.height}this.projectionMatrix.makeOrthographic(o,s,a,c,this.near,this.far,this.coordinateSystem),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){let t=super.toJSON(e);return t.object.zoom=this.zoom,t.object.left=this.left,t.object.right=this.right,t.object.top=this.top,t.object.bottom=this.bottom,t.object.near=this.near,t.object.far=this.far,this.view!==null&&(t.object.view=Object.assign({},this.view)),t}};var gr=new D,Jd=new be,Qd=new be,Bt=class extends Ni{constructor(e=50,t=1,i=.1,n=2e3){super(),this.isPerspectiveCamera=!0,this.type="PerspectiveCamera",this.fov=e,this.zoom=1,this.near=i,this.far=n,this.focus=10,this.aspect=t,this.view=null,this.filmGauge=35,this.filmOffset=0,this.updateProjectionMatrix()}copy(e,t){return super.copy(e,t),this.fov=e.fov,this.zoom=e.zoom,this.near=e.near,this.far=e.far,this.focus=e.focus,this.aspect=e.aspect,this.view=e.view===null?null:Object.assign({},e.view),this.filmGauge=e.filmGauge,this.filmOffset=e.filmOffset,this}setFocalLength(e){let t=.5*this.getFilmHeight()/e;this.fov=Kn*2*Math.atan(t),this.updateProjectionMatrix()}getFocalLength(){let e=Math.tan(zo*.5*this.fov);return .5*this.getFilmHeight()/e}getEffectiveFOV(){return Kn*2*Math.atan(Math.tan(zo*.5*this.fov)/this.zoom)}getFilmWidth(){return this.filmGauge*Math.min(this.aspect,1)}getFilmHeight(){return this.filmGauge/Math.max(this.aspect,1)}getViewBounds(e,t,i){gr.set(-1,-1,.5).applyMatrix4(this.projectionMatrixInverse),t.set(gr.x,gr.y).multiplyScalar(-e/gr.z),gr.set(1,1,.5).applyMatrix4(this.projectionMatrixInverse),i.set(gr.x,gr.y).multiplyScalar(-e/gr.z)}getViewSize(e,t){return this.getViewBounds(e,Jd,Qd),t.subVectors(Qd,Jd)}setViewOffset(e,t,i,n,o,s){this.aspect=e/t,this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=t,this.view.offsetX=i,this.view.offsetY=n,this.view.width=o,this.view.height=s,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){let e=this.near,t=e*Math.tan(zo*.5*this.fov)/this.zoom,i=2*t,n=this.aspect*i,o=-.5*n,s=this.view;if(this.view!==null&&this.view.enabled){let c=s.fullWidth,l=s.fullHeight;o+=s.offsetX*n/c,t-=s.offsetY*i/l,n*=s.width/c,i*=s.height/l}let a=this.filmOffset;a!==0&&(o+=e*a/this.getFilmWidth()),this.projectionMatrix.makePerspective(o,o+n,t,t-i,e,this.far,this.coordinateSystem),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){let t=super.toJSON(e);return t.object.fov=this.fov,t.object.zoom=this.zoom,t.object.near=this.near,t.object.far=this.far,t.object.focus=this.focus,t.object.aspect=this.aspect,this.view!==null&&(t.object.view=Object.assign({},this.view)),t.object.filmGauge=this.filmGauge,t.object.filmOffset=this.filmOffset,t}};var ci=class r extends Tt{constructor(e=1,t=1,i=1,n=1){super(),this.type="PlaneGeometry",this.parameters={width:e,height:t,widthSegments:i,heightSegments:n};let o=e/2,s=t/2,a=Math.floor(i),c=Math.floor(n),l=a+1,d=c+1,u=e/a,h=t/c,p=[],x=[],g=[],m=[];for(let f=0;f<d;f++){let S=f*h-s;for(let _=0;_<l;_++){let R=_*u-o;x.push(R,-S,0),g.push(0,0,1),m.push(_/a),m.push(1-f/c)}}for(let f=0;f<c;f++)for(let S=0;S<a;S++){let _=S+l*f,R=S+l*(f+1),P=S+1+l*(f+1),w=S+1+l*f;p.push(_,R,w),p.push(R,P,w)}this.setIndex(p),this.setAttribute("position",new at(x,3)),this.setAttribute("normal",new at(g,3)),this.setAttribute("uv",new at(m,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new r(e.width,e.height,e.widthSegments,e.heightSegments)}};var lo=class r extends Tt{constructor(e=1,t=1,i=1,n=32,o=1,s=!1,a=0,c=Math.PI*2){super(),this.type="CylinderGeometry",this.parameters={radiusTop:e,radiusBottom:t,height:i,radialSegments:n,heightSegments:o,openEnded:s,thetaStart:a,thetaLength:c};let l=this;n=Math.floor(n),o=Math.floor(o);let d=[],u=[],h=[],p=[],x=0,g=[],m=i/2,f=0;S(),s===!1&&(e>0&&_(!0),t>0&&_(!1)),this.setIndex(d),this.setAttribute("position",new at(u,3)),this.setAttribute("normal",new at(h,3)),this.setAttribute("uv",new at(p,2));function S(){let R=new D,P=new D,w=0,T=(t-e)/i;for(let C=0;C<=o;C++){let y=[],v=C/o,I=v*(t-e)+e;for(let F=0;F<=n;F++){let E=F/n,N=E*c+a,W=Math.sin(N),X=Math.cos(N);P.x=I*W,P.y=-v*i+m,P.z=I*X,u.push(P.x,P.y,P.z),R.set(W,T,X).normalize(),h.push(R.x,R.y,R.z),p.push(E,1-v),y.push(x++)}g.push(y)}for(let C=0;C<n;C++)for(let y=0;y<o;y++){let v=g[y][C],I=g[y+1][C],F=g[y+1][C+1],E=g[y][C+1];d.push(v,I,E),d.push(I,F,E),w+=6}l.addGroup(f,w,0),f+=w}function _(R){let P=x,w=new be,T=new D,C=0,y=R===!0?e:t,v=R===!0?1:-1;for(let F=1;F<=n;F++)u.push(0,m*v,0),h.push(0,v,0),p.push(.5,.5),x++;let I=x;for(let F=0;F<=n;F++){let N=F/n*c+a,W=Math.cos(N),X=Math.sin(N);T.x=y*X,T.y=m*v,T.z=y*W,u.push(T.x,T.y,T.z),h.push(0,v,0),w.x=W*.5+.5,w.y=X*.5*v+.5,p.push(w.x,w.y),x++}for(let F=0;F<n;F++){let E=P+F,N=I+F;R===!0?d.push(N,N+1,E):d.push(N+1,N,E),C+=3}l.addGroup(f,C,R===!0?1:2),f+=C}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new r(e.radiusTop,e.radiusBottom,e.height,e.radialSegments,e.heightSegments,e.openEnded,e.thetaStart,e.thetaLength)}};var eu=new Se().set(.8224621,.177538,0,.0331941,.9668058,0,.0170827,.0723974,.9105199),tu=new Se().set(1.2249401,-.2249404,0,-.0420569,1.0420571,0,-.0196376,-.0786361,1.0982735),us={[ei]:{transfer:en,primaries:tn,toReference:r=>r,fromReference:r=>r},[At]:{transfer:$e,primaries:tn,toReference:r=>r.convertSRGBToLinear(),fromReference:r=>r.convertLinearToSRGB()},[Ir]:{transfer:en,primaries:rn,toReference:r=>r.applyMatrix3(tu),fromReference:r=>r.applyMatrix3(eu)},[Qr]:{transfer:$e,primaries:rn,toReference:r=>r.convertSRGBToLinear().applyMatrix3(tu),fromReference:r=>r.applyMatrix3(eu).convertLinearToSRGB()}},Eg=new Set([ei,Ir]),Oe={enabled:!0,_workingColorSpace:ei,get workingColorSpace(){return this._workingColorSpace},set workingColorSpace(r){if(!Eg.has(r))throw new Error(`Unsupported working color space, "${r}".`);this._workingColorSpace=r},convert:function(r,e,t){if(this.enabled===!1||e===t||!e||!t)return r;let i=us[e].toReference,n=us[t].fromReference;return n(i(r))},fromWorkingColorSpace:function(r,e){return this.convert(r,this._workingColorSpace,e)},toWorkingColorSpace:function(r,e){return this.convert(r,e,this._workingColorSpace)},getPrimaries:function(r){return us[r].primaries},getTransfer:function(r){return r===hi?en:us[r].transfer}};function hs(r){return r<.04045?r*.0773993808:Math.pow(r*.9478672986+.0521327014,2.4)}function fs(r){return r<.0031308?r*12.92:1.055*Math.pow(r,.41666)-.055}var iu={aliceblue:15792383,antiquewhite:16444375,aqua:65535,aquamarine:8388564,azure:15794175,beige:16119260,bisque:16770244,black:0,blanchedalmond:16772045,blue:255,blueviolet:9055202,brown:10824234,burlywood:14596231,cadetblue:6266528,chartreuse:8388352,chocolate:13789470,coral:16744272,cornflowerblue:6591981,cornsilk:16775388,crimson:14423100,cyan:65535,darkblue:139,darkcyan:35723,darkgoldenrod:12092939,darkgray:11119017,darkgreen:25600,darkgrey:11119017,darkkhaki:12433259,darkmagenta:9109643,darkolivegreen:5597999,darkorange:16747520,darkorchid:10040012,darkred:9109504,darksalmon:15308410,darkseagreen:9419919,darkslateblue:4734347,darkslategray:3100495,darkslategrey:3100495,darkturquoise:52945,darkviolet:9699539,deeppink:16716947,deepskyblue:49151,dimgray:6908265,dimgrey:6908265,dodgerblue:2003199,firebrick:11674146,floralwhite:16775920,forestgreen:2263842,fuchsia:16711935,gainsboro:14474460,ghostwhite:16316671,gold:16766720,goldenrod:14329120,gray:8421504,green:32768,greenyellow:11403055,grey:8421504,honeydew:15794160,hotpink:16738740,indianred:13458524,indigo:4915330,ivory:16777200,khaki:15787660,lavender:15132410,lavenderblush:16773365,lawngreen:8190976,lemonchiffon:16775885,lightblue:11393254,lightcoral:15761536,lightcyan:14745599,lightgoldenrodyellow:16448210,lightgray:13882323,lightgreen:9498256,lightgrey:13882323,lightpink:16758465,lightsalmon:16752762,lightseagreen:2142890,lightskyblue:8900346,lightslategray:7833753,lightslategrey:7833753,lightsteelblue:11584734,lightyellow:16777184,lime:65280,limegreen:3329330,linen:16445670,magenta:16711935,maroon:8388608,mediumaquamarine:6737322,mediumblue:205,mediumorchid:12211667,mediumpurple:9662683,mediumseagreen:3978097,mediumslateblue:8087790,mediumspringgreen:64154,mediumturquoise:4772300,mediumvioletred:13047173,midnightblue:1644912,mintcream:16121850,mistyrose:16770273,moccasin:16770229,navajowhite:16768685,navy:128,oldlace:16643558,olive:8421376,olivedrab:7048739,orange:16753920,orangered:16729344,orchid:14315734,palegoldenrod:15657130,palegreen:10025880,paleturquoise:11529966,palevioletred:14381203,papayawhip:16773077,peachpuff:16767673,peru:13468991,pink:16761035,plum:14524637,powderblue:11591910,purple:8388736,rebeccapurple:6697881,red:16711680,rosybrown:12357519,royalblue:4286945,saddlebrown:9127187,salmon:16416882,sandybrown:16032864,seagreen:3050327,seashell:16774638,sienna:10506797,silver:12632256,skyblue:8900331,slateblue:6970061,slategray:7372944,slategrey:7372944,snow:16775930,springgreen:65407,steelblue:4620980,tan:13808780,teal:32896,thistle:14204888,tomato:16737095,turquoise:4251856,violet:15631086,wheat:16113331,white:16777215,whitesmoke:16119285,yellow:16776960,yellowgreen:10145074},xr={h:0,s:0,l:0},ps={h:0,s:0,l:0};function rc(r,e,t){return t<0&&(t+=1),t>1&&(t-=1),t<1/6?r+(e-r)*6*t:t<1/2?e:t<2/3?r+(e-r)*6*(2/3-t):r}var Me=class{constructor(e,t,i){return this.isColor=!0,this.r=1,this.g=1,this.b=1,this.set(e,t,i)}set(e,t,i){if(t===void 0&&i===void 0){let n=e;n&&n.isColor?this.copy(n):typeof n=="number"?this.setHex(n):typeof n=="string"&&this.setStyle(n)}else this.setRGB(e,t,i);return this}setScalar(e){return this.r=e,this.g=e,this.b=e,this}setHex(e,t=At){return e=Math.floor(e),this.r=(e>>16&255)/255,this.g=(e>>8&255)/255,this.b=(e&255)/255,Oe.toWorkingColorSpace(this,t),this}setRGB(e,t,i,n=Oe.workingColorSpace){return this.r=e,this.g=t,this.b=i,Oe.toWorkingColorSpace(this,n),this}setHSL(e,t,i,n=Oe.workingColorSpace){if(e=Fl(e,1),t=Et(t,0,1),i=Et(i,0,1),t===0)this.r=this.g=this.b=i;else{let o=i<=.5?i*(1+t):i+t-i*t,s=2*i-o;this.r=rc(s,o,e+1/3),this.g=rc(s,o,e),this.b=rc(s,o,e-1/3)}return Oe.toWorkingColorSpace(this,n),this}setStyle(e,t=At){function i(o){o!==void 0&&parseFloat(o)<1&&console.warn("THREE.Color: Alpha component of "+e+" will be ignored.")}let n;if(n=/^(\w+)\(([^\)]*)\)/.exec(e)){let o,s=n[1],a=n[2];switch(s){case"rgb":case"rgba":if(o=/^\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(a))return i(o[4]),this.setRGB(Math.min(255,parseInt(o[1],10))/255,Math.min(255,parseInt(o[2],10))/255,Math.min(255,parseInt(o[3],10))/255,t);if(o=/^\s*(\d+)\%\s*,\s*(\d+)\%\s*,\s*(\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(a))return i(o[4]),this.setRGB(Math.min(100,parseInt(o[1],10))/100,Math.min(100,parseInt(o[2],10))/100,Math.min(100,parseInt(o[3],10))/100,t);break;case"hsl":case"hsla":if(o=/^\s*(\d*\.?\d+)\s*,\s*(\d*\.?\d+)\%\s*,\s*(\d*\.?\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec(a))return i(o[4]),this.setHSL(parseFloat(o[1])/360,parseFloat(o[2])/100,parseFloat(o[3])/100,t);break;default:console.warn("THREE.Color: Unknown color model "+e)}}else if(n=/^\#([A-Fa-f\d]+)$/.exec(e)){let o=n[1],s=o.length;if(s===3)return this.setRGB(parseInt(o.charAt(0),16)/15,parseInt(o.charAt(1),16)/15,parseInt(o.charAt(2),16)/15,t);if(s===6)return this.setHex(parseInt(o,16),t);console.warn("THREE.Color: Invalid hex color "+e)}else if(e&&e.length>0)return this.setColorName(e,t);return this}setColorName(e,t=At){let i=iu[e.toLowerCase()];return i!==void 0?this.setHex(i,t):console.warn("THREE.Color: Unknown color "+e),this}clone(){return new this.constructor(this.r,this.g,this.b)}copy(e){return this.r=e.r,this.g=e.g,this.b=e.b,this}copySRGBToLinear(e){return this.r=hs(e.r),this.g=hs(e.g),this.b=hs(e.b),this}copyLinearToSRGB(e){return this.r=fs(e.r),this.g=fs(e.g),this.b=fs(e.b),this}convertSRGBToLinear(){return this.copySRGBToLinear(this),this}convertLinearToSRGB(){return this.copyLinearToSRGB(this),this}getHex(e=At){return Oe.fromWorkingColorSpace(Ht.copy(this),e),Math.round(Et(Ht.r*255,0,255))*65536+Math.round(Et(Ht.g*255,0,255))*256+Math.round(Et(Ht.b*255,0,255))}getHexString(e=At){return("000000"+this.getHex(e).toString(16)).slice(-6)}getHSL(e,t=Oe.workingColorSpace){Oe.fromWorkingColorSpace(Ht.copy(this),t);let i=Ht.r,n=Ht.g,o=Ht.b,s=Math.max(i,n,o),a=Math.min(i,n,o),c,l,d=(a+s)/2;if(a===s)c=0,l=0;else{let u=s-a;switch(l=d<=.5?u/(s+a):u/(2-s-a),s){case i:c=(n-o)/u+(n<o?6:0);break;case n:c=(o-i)/u+2;break;case o:c=(i-n)/u+4;break}c/=6}return e.h=c,e.s=l,e.l=d,e}getRGB(e,t=Oe.workingColorSpace){return Oe.fromWorkingColorSpace(Ht.copy(this),t),e.r=Ht.r,e.g=Ht.g,e.b=Ht.b,e}getStyle(e=At){Oe.fromWorkingColorSpace(Ht.copy(this),e);let t=Ht.r,i=Ht.g,n=Ht.b;return e!==At?`color(${e} ${t.toFixed(3)} ${i.toFixed(3)} ${n.toFixed(3)})`:`rgb(${Math.round(t*255)},${Math.round(i*255)},${Math.round(n*255)})`}offsetHSL(e,t,i){return this.getHSL(xr),this.setHSL(xr.h+e,xr.s+t,xr.l+i)}add(e){return this.r+=e.r,this.g+=e.g,this.b+=e.b,this}addColors(e,t){return this.r=e.r+t.r,this.g=e.g+t.g,this.b=e.b+t.b,this}addScalar(e){return this.r+=e,this.g+=e,this.b+=e,this}sub(e){return this.r=Math.max(0,this.r-e.r),this.g=Math.max(0,this.g-e.g),this.b=Math.max(0,this.b-e.b),this}multiply(e){return this.r*=e.r,this.g*=e.g,this.b*=e.b,this}multiplyScalar(e){return this.r*=e,this.g*=e,this.b*=e,this}lerp(e,t){return this.r+=(e.r-this.r)*t,this.g+=(e.g-this.g)*t,this.b+=(e.b-this.b)*t,this}lerpColors(e,t,i){return this.r=e.r+(t.r-e.r)*i,this.g=e.g+(t.g-e.g)*i,this.b=e.b+(t.b-e.b)*i,this}lerpHSL(e,t){this.getHSL(xr),e.getHSL(ps);let i=Vo(xr.h,ps.h,t),n=Vo(xr.s,ps.s,t),o=Vo(xr.l,ps.l,t);return this.setHSL(i,n,o),this}setFromVector3(e){return this.r=e.x,this.g=e.y,this.b=e.z,this}applyMatrix3(e){let t=this.r,i=this.g,n=this.b,o=e.elements;return this.r=o[0]*t+o[3]*i+o[6]*n,this.g=o[1]*t+o[4]*i+o[7]*n,this.b=o[2]*t+o[5]*i+o[8]*n,this}equals(e){return e.r===this.r&&e.g===this.g&&e.b===this.b}fromArray(e,t=0){return this.r=e[t],this.g=e[t+1],this.b=e[t+2],this}toArray(e=[],t=0){return e[t]=this.r,e[t+1]=this.g,e[t+2]=this.b,e}fromBufferAttribute(e,t){return this.r=e.getX(t),this.g=e.getY(t),this.b=e.getZ(t),this}toJSON(){return this.getHex()}*[Symbol.iterator](){yield this.r,yield this.g,yield this.b}},Ht=new Me;Me.NAMES=iu;var Ag=0,Bi=class extends Qt{constructor(){super(),this.isMaterial=!0,Object.defineProperty(this,"id",{value:Ag++}),this.uuid=Di(),this.name="",this.type="Material",this.blending=Xi,this.side=ni,this.vertexColors=!1,this.opacity=1,this.transparent=!1,this.alphaHash=!1,this.blendSrc=Qn,this.blendDst=eo,this.blendEquation=qi,this.blendSrcAlpha=null,this.blendDstAlpha=null,this.blendEquationAlpha=null,this.blendColor=new Me(0,0,0),this.blendAlpha=0,this.depthFunc=Jr,this.depthTest=!0,this.depthWrite=!0,this.stencilWriteMask=255,this.stencilFunc=Ka,this.stencilRef=0,this.stencilFuncMask=255,this.stencilFail=Dr,this.stencilZFail=Dr,this.stencilZPass=Dr,this.stencilWrite=!1,this.clippingPlanes=null,this.clipIntersection=!1,this.clipShadows=!1,this.shadowSide=null,this.colorWrite=!0,this.precision=null,this.polygonOffset=!1,this.polygonOffsetFactor=0,this.polygonOffsetUnits=0,this.dithering=!1,this.alphaToCoverage=!1,this.premultipliedAlpha=!1,this.forceSinglePass=!1,this.visible=!0,this.toneMapped=!0,this.userData={},this.version=0,this._alphaTest=0}get alphaTest(){return this._alphaTest}set alphaTest(e){this._alphaTest>0!=e>0&&this.version++,this._alphaTest=e}onBuild(){}onBeforeRender(){}onBeforeCompile(){}customProgramCacheKey(){return this.onBeforeCompile.toString()}setValues(e){if(e!==void 0)for(let t in e){let i=e[t];if(i===void 0){console.warn(`THREE.Material: parameter '${t}' has value of undefined.`);continue}let n=this[t];if(n===void 0){console.warn(`THREE.Material: '${t}' is not a property of THREE.${this.type}.`);continue}n&&n.isColor?n.set(i):n&&n.isVector3&&i&&i.isVector3?n.copy(i):this[t]=i}}toJSON(e){let t=e===void 0||typeof e=="string";t&&(e={textures:{},images:{}});let i={metadata:{version:4.6,type:"Material",generator:"Material.toJSON"}};i.uuid=this.uuid,i.type=this.type,this.name!==""&&(i.name=this.name),this.color&&this.color.isColor&&(i.color=this.color.getHex()),this.roughness!==void 0&&(i.roughness=this.roughness),this.metalness!==void 0&&(i.metalness=this.metalness),this.sheen!==void 0&&(i.sheen=this.sheen),this.sheenColor&&this.sheenColor.isColor&&(i.sheenColor=this.sheenColor.getHex()),this.sheenRoughness!==void 0&&(i.sheenRoughness=this.sheenRoughness),this.emissive&&this.emissive.isColor&&(i.emissive=this.emissive.getHex()),this.emissiveIntensity!==void 0&&this.emissiveIntensity!==1&&(i.emissiveIntensity=this.emissiveIntensity),this.specular&&this.specular.isColor&&(i.specular=this.specular.getHex()),this.specularIntensity!==void 0&&(i.specularIntensity=this.specularIntensity),this.specularColor&&this.specularColor.isColor&&(i.specularColor=this.specularColor.getHex()),this.shininess!==void 0&&(i.shininess=this.shininess),this.clearcoat!==void 0&&(i.clearcoat=this.clearcoat),this.clearcoatRoughness!==void 0&&(i.clearcoatRoughness=this.clearcoatRoughness),this.clearcoatMap&&this.clearcoatMap.isTexture&&(i.clearcoatMap=this.clearcoatMap.toJSON(e).uuid),this.clearcoatRoughnessMap&&this.clearcoatRoughnessMap.isTexture&&(i.clearcoatRoughnessMap=this.clearcoatRoughnessMap.toJSON(e).uuid),this.clearcoatNormalMap&&this.clearcoatNormalMap.isTexture&&(i.clearcoatNormalMap=this.clearcoatNormalMap.toJSON(e).uuid,i.clearcoatNormalScale=this.clearcoatNormalScale.toArray()),this.dispersion!==void 0&&(i.dispersion=this.dispersion),this.iridescence!==void 0&&(i.iridescence=this.iridescence),this.iridescenceIOR!==void 0&&(i.iridescenceIOR=this.iridescenceIOR),this.iridescenceThicknessRange!==void 0&&(i.iridescenceThicknessRange=this.iridescenceThicknessRange),this.iridescenceMap&&this.iridescenceMap.isTexture&&(i.iridescenceMap=this.iridescenceMap.toJSON(e).uuid),this.iridescenceThicknessMap&&this.iridescenceThicknessMap.isTexture&&(i.iridescenceThicknessMap=this.iridescenceThicknessMap.toJSON(e).uuid),this.anisotropy!==void 0&&(i.anisotropy=this.anisotropy),this.anisotropyRotation!==void 0&&(i.anisotropyRotation=this.anisotropyRotation),this.anisotropyMap&&this.anisotropyMap.isTexture&&(i.anisotropyMap=this.anisotropyMap.toJSON(e).uuid),this.map&&this.map.isTexture&&(i.map=this.map.toJSON(e).uuid),this.matcap&&this.matcap.isTexture&&(i.matcap=this.matcap.toJSON(e).uuid),this.alphaMap&&this.alphaMap.isTexture&&(i.alphaMap=this.alphaMap.toJSON(e).uuid),this.lightMap&&this.lightMap.isTexture&&(i.lightMap=this.lightMap.toJSON(e).uuid,i.lightMapIntensity=this.lightMapIntensity),this.aoMap&&this.aoMap.isTexture&&(i.aoMap=this.aoMap.toJSON(e).uuid,i.aoMapIntensity=this.aoMapIntensity),this.bumpMap&&this.bumpMap.isTexture&&(i.bumpMap=this.bumpMap.toJSON(e).uuid,i.bumpScale=this.bumpScale),this.normalMap&&this.normalMap.isTexture&&(i.normalMap=this.normalMap.toJSON(e).uuid,i.normalMapType=this.normalMapType,i.normalScale=this.normalScale.toArray()),this.displacementMap&&this.displacementMap.isTexture&&(i.displacementMap=this.displacementMap.toJSON(e).uuid,i.displacementScale=this.displacementScale,i.displacementBias=this.displacementBias),this.roughnessMap&&this.roughnessMap.isTexture&&(i.roughnessMap=this.roughnessMap.toJSON(e).uuid),this.metalnessMap&&this.metalnessMap.isTexture&&(i.metalnessMap=this.metalnessMap.toJSON(e).uuid),this.emissiveMap&&this.emissiveMap.isTexture&&(i.emissiveMap=this.emissiveMap.toJSON(e).uuid),this.specularMap&&this.specularMap.isTexture&&(i.specularMap=this.specularMap.toJSON(e).uuid),this.specularIntensityMap&&this.specularIntensityMap.isTexture&&(i.specularIntensityMap=this.specularIntensityMap.toJSON(e).uuid),this.specularColorMap&&this.specularColorMap.isTexture&&(i.specularColorMap=this.specularColorMap.toJSON(e).uuid),this.envMap&&this.envMap.isTexture&&(i.envMap=this.envMap.toJSON(e).uuid,this.combine!==void 0&&(i.combine=this.combine)),this.envMapRotation!==void 0&&(i.envMapRotation=this.envMapRotation.toArray()),this.envMapIntensity!==void 0&&(i.envMapIntensity=this.envMapIntensity),this.reflectivity!==void 0&&(i.reflectivity=this.reflectivity),this.refractionRatio!==void 0&&(i.refractionRatio=this.refractionRatio),this.gradientMap&&this.gradientMap.isTexture&&(i.gradientMap=this.gradientMap.toJSON(e).uuid),this.transmission!==void 0&&(i.transmission=this.transmission),this.transmissionMap&&this.transmissionMap.isTexture&&(i.transmissionMap=this.transmissionMap.toJSON(e).uuid),this.thickness!==void 0&&(i.thickness=this.thickness),this.thicknessMap&&this.thicknessMap.isTexture&&(i.thicknessMap=this.thicknessMap.toJSON(e).uuid),this.attenuationDistance!==void 0&&this.attenuationDistance!==1/0&&(i.attenuationDistance=this.attenuationDistance),this.attenuationColor!==void 0&&(i.attenuationColor=this.attenuationColor.getHex()),this.size!==void 0&&(i.size=this.size),this.shadowSide!==null&&(i.shadowSide=this.shadowSide),this.sizeAttenuation!==void 0&&(i.sizeAttenuation=this.sizeAttenuation),this.blending!==Xi&&(i.blending=this.blending),this.side!==ni&&(i.side=this.side),this.vertexColors===!0&&(i.vertexColors=!0),this.opacity<1&&(i.opacity=this.opacity),this.transparent===!0&&(i.transparent=!0),this.blendSrc!==Qn&&(i.blendSrc=this.blendSrc),this.blendDst!==eo&&(i.blendDst=this.blendDst),this.blendEquation!==qi&&(i.blendEquation=this.blendEquation),this.blendSrcAlpha!==null&&(i.blendSrcAlpha=this.blendSrcAlpha),this.blendDstAlpha!==null&&(i.blendDstAlpha=this.blendDstAlpha),this.blendEquationAlpha!==null&&(i.blendEquationAlpha=this.blendEquationAlpha),this.blendColor&&this.blendColor.isColor&&(i.blendColor=this.blendColor.getHex()),this.blendAlpha!==0&&(i.blendAlpha=this.blendAlpha),this.depthFunc!==Jr&&(i.depthFunc=this.depthFunc),this.depthTest===!1&&(i.depthTest=this.depthTest),this.depthWrite===!1&&(i.depthWrite=this.depthWrite),this.colorWrite===!1&&(i.colorWrite=this.colorWrite),this.stencilWriteMask!==255&&(i.stencilWriteMask=this.stencilWriteMask),this.stencilFunc!==Ka&&(i.stencilFunc=this.stencilFunc),this.stencilRef!==0&&(i.stencilRef=this.stencilRef),this.stencilFuncMask!==255&&(i.stencilFuncMask=this.stencilFuncMask),this.stencilFail!==Dr&&(i.stencilFail=this.stencilFail),this.stencilZFail!==Dr&&(i.stencilZFail=this.stencilZFail),this.stencilZPass!==Dr&&(i.stencilZPass=this.stencilZPass),this.stencilWrite===!0&&(i.stencilWrite=this.stencilWrite),this.rotation!==void 0&&this.rotation!==0&&(i.rotation=this.rotation),this.polygonOffset===!0&&(i.polygonOffset=!0),this.polygonOffsetFactor!==0&&(i.polygonOffsetFactor=this.polygonOffsetFactor),this.polygonOffsetUnits!==0&&(i.polygonOffsetUnits=this.polygonOffsetUnits),this.linewidth!==void 0&&this.linewidth!==1&&(i.linewidth=this.linewidth),this.dashSize!==void 0&&(i.dashSize=this.dashSize),this.gapSize!==void 0&&(i.gapSize=this.gapSize),this.scale!==void 0&&(i.scale=this.scale),this.dithering===!0&&(i.dithering=!0),this.alphaTest>0&&(i.alphaTest=this.alphaTest),this.alphaHash===!0&&(i.alphaHash=!0),this.alphaToCoverage===!0&&(i.alphaToCoverage=!0),this.premultipliedAlpha===!0&&(i.premultipliedAlpha=!0),this.forceSinglePass===!0&&(i.forceSinglePass=!0),this.wireframe===!0&&(i.wireframe=!0),this.wireframeLinewidth>1&&(i.wireframeLinewidth=this.wireframeLinewidth),this.wireframeLinecap!=="round"&&(i.wireframeLinecap=this.wireframeLinecap),this.wireframeLinejoin!=="round"&&(i.wireframeLinejoin=this.wireframeLinejoin),this.flatShading===!0&&(i.flatShading=!0),this.visible===!1&&(i.visible=!1),this.toneMapped===!1&&(i.toneMapped=!1),this.fog===!1&&(i.fog=!1),Object.keys(this.userData).length>0&&(i.userData=this.userData);function n(o){let s=[];for(let a in o){let c=o[a];delete c.metadata,s.push(c)}return s}if(t){let o=n(e.textures),s=n(e.images);o.length>0&&(i.textures=o),s.length>0&&(i.images=s)}return i}clone(){return new this.constructor().copy(this)}copy(e){this.name=e.name,this.blending=e.blending,this.side=e.side,this.vertexColors=e.vertexColors,this.opacity=e.opacity,this.transparent=e.transparent,this.blendSrc=e.blendSrc,this.blendDst=e.blendDst,this.blendEquation=e.blendEquation,this.blendSrcAlpha=e.blendSrcAlpha,this.blendDstAlpha=e.blendDstAlpha,this.blendEquationAlpha=e.blendEquationAlpha,this.blendColor.copy(e.blendColor),this.blendAlpha=e.blendAlpha,this.depthFunc=e.depthFunc,this.depthTest=e.depthTest,this.depthWrite=e.depthWrite,this.stencilWriteMask=e.stencilWriteMask,this.stencilFunc=e.stencilFunc,this.stencilRef=e.stencilRef,this.stencilFuncMask=e.stencilFuncMask,this.stencilFail=e.stencilFail,this.stencilZFail=e.stencilZFail,this.stencilZPass=e.stencilZPass,this.stencilWrite=e.stencilWrite;let t=e.clippingPlanes,i=null;if(t!==null){let n=t.length;i=new Array(n);for(let o=0;o!==n;++o)i[o]=t[o].clone()}return this.clippingPlanes=i,this.clipIntersection=e.clipIntersection,this.clipShadows=e.clipShadows,this.shadowSide=e.shadowSide,this.colorWrite=e.colorWrite,this.precision=e.precision,this.polygonOffset=e.polygonOffset,this.polygonOffsetFactor=e.polygonOffsetFactor,this.polygonOffsetUnits=e.polygonOffsetUnits,this.dithering=e.dithering,this.alphaTest=e.alphaTest,this.alphaHash=e.alphaHash,this.alphaToCoverage=e.alphaToCoverage,this.premultipliedAlpha=e.premultipliedAlpha,this.forceSinglePass=e.forceSinglePass,this.visible=e.visible,this.toneMapped=e.toneMapped,this.userData=JSON.parse(JSON.stringify(e.userData)),this}dispose(){this.dispatchEvent({type:"dispose"})}set needsUpdate(e){e===!0&&this.version++}};function Ki(r){let e={};for(let t in r){e[t]={};for(let i in r[t]){let n=r[t][i];n&&(n.isColor||n.isMatrix3||n.isMatrix4||n.isVector2||n.isVector3||n.isVector4||n.isTexture||n.isQuaternion)?n.isRenderTargetTexture?(console.warn("UniformsUtils: Textures of render targets cannot be cloned via cloneUniforms() or mergeUniforms()."),e[t][i]=null):e[t][i]=n.clone():Array.isArray(n)?e[t][i]=n.slice():e[t][i]=n}}return e}function jt(r){let e={};for(let t=0;t<r.length;t++){let i=Ki(r[t]);for(let n in i)e[n]=i[n]}return e}function ru(r){let e=[];for(let t=0;t<r.length;t++)e.push(r[t].clone());return e}function ms(r){let e=r.getRenderTarget();return e===null?r.outputColorSpace:e.isXRRenderTarget===!0?e.texture.colorSpace:Oe.workingColorSpace}var uo={clone:Ki,merge:jt};var nu=`
void main() {
	gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
}
`;var ou=`
void main() {
	gl_FragColor = vec4( 1.0, 0.0, 0.0, 1.0 );
}
`;var ft=class extends Bi{constructor(e){super(),this.isShaderMaterial=!0,this.type="ShaderMaterial",this.defines={},this.uniforms={},this.uniformsGroups=[],this.vertexShader=nu,this.fragmentShader=ou,this.linewidth=1,this.wireframe=!1,this.wireframeLinewidth=1,this.fog=!1,this.lights=!1,this.clipping=!1,this.forceSinglePass=!0,this.extensions={clipCullDistance:!1,multiDraw:!1},this.defaultAttributeValues={color:[1,1,1],uv:[0,0],uv1:[0,0]},this.index0AttributeName=void 0,this.uniformsNeedUpdate=!1,this.glslVersion=null,e!==void 0&&this.setValues(e)}copy(e){return super.copy(e),this.fragmentShader=e.fragmentShader,this.vertexShader=e.vertexShader,this.uniforms=Ki(e.uniforms),this.uniformsGroups=ru(e.uniformsGroups),this.defines=Object.assign({},e.defines),this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.fog=e.fog,this.lights=e.lights,this.clipping=e.clipping,this.extensions=Object.assign({},e.extensions),this.glslVersion=e.glslVersion,this}toJSON(e){let t=super.toJSON(e);t.glslVersion=this.glslVersion,t.uniforms={};for(let n in this.uniforms){let s=this.uniforms[n].value;s&&s.isTexture?t.uniforms[n]={type:"t",value:s.toJSON(e).uuid}:s&&s.isColor?t.uniforms[n]={type:"c",value:s.getHex()}:s&&s.isVector2?t.uniforms[n]={type:"v2",value:s.toArray()}:s&&s.isVector3?t.uniforms[n]={type:"v3",value:s.toArray()}:s&&s.isVector4?t.uniforms[n]={type:"v4",value:s.toArray()}:s&&s.isMatrix3?t.uniforms[n]={type:"m3",value:s.toArray()}:s&&s.isMatrix4?t.uniforms[n]={type:"m4",value:s.toArray()}:t.uniforms[n]={value:s}}Object.keys(this.defines).length>0&&(t.defines=this.defines),t.vertexShader=this.vertexShader,t.fragmentShader=this.fragmentShader,t.lights=this.lights,t.clipping=this.clipping;let i={};for(let n in this.extensions)this.extensions[n]===!0&&(i[n]=!0);return Object.keys(i).length>0&&(t.extensions=i),t}};var bi=class extends Bi{constructor(e){super(),this.isMeshBasicMaterial=!0,this.type="MeshBasicMaterial",this.color=new Me(16777215),this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.specularMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new ii,this.combine=Xo,this.reflectivity=1,this.refractionRatio=.98,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.color.copy(e.color),this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.specularMap=e.specularMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.combine=e.combine,this.reflectivity=e.reflectivity,this.refractionRatio=e.refractionRatio,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.fog=e.fog,this}};var Zi=new D,nc=new D,gs=new D,vr=new D,oc=new D,xs=new D,sc=new D,vs=class{constructor(e=new D,t=new D(0,0,-1)){this.origin=e,this.direction=t}set(e,t){return this.origin.copy(e),this.direction.copy(t),this}copy(e){return this.origin.copy(e.origin),this.direction.copy(e.direction),this}at(e,t){return t.copy(this.origin).addScaledVector(this.direction,e)}lookAt(e){return this.direction.copy(e).sub(this.origin).normalize(),this}recast(e){return this.origin.copy(this.at(e,Zi)),this}closestPointToPoint(e,t){t.subVectors(e,this.origin);let i=t.dot(this.direction);return i<0?t.copy(this.origin):t.copy(this.origin).addScaledVector(this.direction,i)}distanceToPoint(e){return Math.sqrt(this.distanceSqToPoint(e))}distanceSqToPoint(e){let t=Zi.subVectors(e,this.origin).dot(this.direction);return t<0?this.origin.distanceToSquared(e):(Zi.copy(this.origin).addScaledVector(this.direction,t),Zi.distanceToSquared(e))}distanceSqToSegment(e,t,i,n){nc.copy(e).add(t).multiplyScalar(.5),gs.copy(t).sub(e).normalize(),vr.copy(this.origin).sub(nc);let o=e.distanceTo(t)*.5,s=-this.direction.dot(gs),a=vr.dot(this.direction),c=-vr.dot(gs),l=vr.lengthSq(),d=Math.abs(1-s*s),u,h,p,x;if(d>0)if(u=s*c-a,h=s*a-c,x=o*d,u>=0)if(h>=-x)if(h<=x){let g=1/d;u*=g,h*=g,p=u*(u+s*h+2*a)+h*(s*u+h+2*c)+l}else h=o,u=Math.max(0,-(s*h+a)),p=-u*u+h*(h+2*c)+l;else h=-o,u=Math.max(0,-(s*h+a)),p=-u*u+h*(h+2*c)+l;else h<=-x?(u=Math.max(0,-(-s*o+a)),h=u>0?-o:Math.min(Math.max(-o,-c),o),p=-u*u+h*(h+2*c)+l):h<=x?(u=0,h=Math.min(Math.max(-o,-c),o),p=h*(h+2*c)+l):(u=Math.max(0,-(s*o+a)),h=u>0?o:Math.min(Math.max(-o,-c),o),p=-u*u+h*(h+2*c)+l);else h=s>0?-o:o,u=Math.max(0,-(s*h+a)),p=-u*u+h*(h+2*c)+l;return i&&i.copy(this.origin).addScaledVector(this.direction,u),n&&n.copy(nc).addScaledVector(gs,h),p}intersectSphere(e,t){Zi.subVectors(e.center,this.origin);let i=Zi.dot(this.direction),n=Zi.dot(Zi)-i*i,o=e.radius*e.radius;if(n>o)return null;let s=Math.sqrt(o-n),a=i-s,c=i+s;return c<0?null:a<0?this.at(c,t):this.at(a,t)}intersectsSphere(e){return this.distanceSqToPoint(e.center)<=e.radius*e.radius}distanceToPlane(e){let t=e.normal.dot(this.direction);if(t===0)return e.distanceToPoint(this.origin)===0?0:null;let i=-(this.origin.dot(e.normal)+e.constant)/t;return i>=0?i:null}intersectPlane(e,t){let i=this.distanceToPlane(e);return i===null?null:this.at(i,t)}intersectsPlane(e){let t=e.distanceToPoint(this.origin);return t===0||e.normal.dot(this.direction)*t<0}intersectBox(e,t){let i,n,o,s,a,c,l=1/this.direction.x,d=1/this.direction.y,u=1/this.direction.z,h=this.origin;return l>=0?(i=(e.min.x-h.x)*l,n=(e.max.x-h.x)*l):(i=(e.max.x-h.x)*l,n=(e.min.x-h.x)*l),d>=0?(o=(e.min.y-h.y)*d,s=(e.max.y-h.y)*d):(o=(e.max.y-h.y)*d,s=(e.min.y-h.y)*d),i>s||o>n||((o>i||isNaN(i))&&(i=o),(s<n||isNaN(n))&&(n=s),u>=0?(a=(e.min.z-h.z)*u,c=(e.max.z-h.z)*u):(a=(e.max.z-h.z)*u,c=(e.min.z-h.z)*u),i>c||a>n)||((a>i||i!==i)&&(i=a),(c<n||n!==n)&&(n=c),n<0)?null:this.at(i>=0?i:n,t)}intersectsBox(e){return this.intersectBox(e,Zi)!==null}intersectTriangle(e,t,i,n,o){oc.subVectors(t,e),xs.subVectors(i,e),sc.crossVectors(oc,xs);let s=this.direction.dot(sc),a;if(s>0){if(n)return null;a=1}else if(s<0)a=-1,s=-s;else return null;vr.subVectors(this.origin,e);let c=a*this.direction.dot(xs.crossVectors(vr,xs));if(c<0)return null;let l=a*this.direction.dot(oc.cross(vr));if(l<0||c+l>s)return null;let d=-a*vr.dot(sc);return d<0?null:this.at(d/s,o)}applyMatrix4(e){return this.origin.applyMatrix4(e),this.direction.transformDirection(e),this}equals(e){return e.origin.equals(this.origin)&&e.direction.equals(this.direction)}clone(){return new this.constructor().copy(this)}};var Ri=new D,Ji=new D,ac=new D,Qi=new D,hn=new D,fn=new D,su=new D,cc=new D,lc=new D,dc=new D,Ur=class r{constructor(e=new D,t=new D,i=new D){this.a=e,this.b=t,this.c=i}static getNormal(e,t,i,n){n.subVectors(i,t),Ri.subVectors(e,t),n.cross(Ri);let o=n.lengthSq();return o>0?n.multiplyScalar(1/Math.sqrt(o)):n.set(0,0,0)}static getBarycoord(e,t,i,n,o){Ri.subVectors(n,t),Ji.subVectors(i,t),ac.subVectors(e,t);let s=Ri.dot(Ri),a=Ri.dot(Ji),c=Ri.dot(ac),l=Ji.dot(Ji),d=Ji.dot(ac),u=s*l-a*a;if(u===0)return o.set(0,0,0),null;let h=1/u,p=(l*c-a*d)*h,x=(s*d-a*c)*h;return o.set(1-p-x,x,p)}static containsPoint(e,t,i,n){return this.getBarycoord(e,t,i,n,Qi)===null?!1:Qi.x>=0&&Qi.y>=0&&Qi.x+Qi.y<=1}static getInterpolation(e,t,i,n,o,s,a,c){return this.getBarycoord(e,t,i,n,Qi)===null?(c.x=0,c.y=0,"z"in c&&(c.z=0),"w"in c&&(c.w=0),null):(c.setScalar(0),c.addScaledVector(o,Qi.x),c.addScaledVector(s,Qi.y),c.addScaledVector(a,Qi.z),c)}static isFrontFacing(e,t,i,n){return Ri.subVectors(i,t),Ji.subVectors(e,t),Ri.cross(Ji).dot(n)<0}set(e,t,i){return this.a.copy(e),this.b.copy(t),this.c.copy(i),this}setFromPointsAndIndices(e,t,i,n){return this.a.copy(e[t]),this.b.copy(e[i]),this.c.copy(e[n]),this}setFromAttributeAndIndices(e,t,i,n){return this.a.fromBufferAttribute(e,t),this.b.fromBufferAttribute(e,i),this.c.fromBufferAttribute(e,n),this}clone(){return new this.constructor().copy(this)}copy(e){return this.a.copy(e.a),this.b.copy(e.b),this.c.copy(e.c),this}getArea(){return Ri.subVectors(this.c,this.b),Ji.subVectors(this.a,this.b),Ri.cross(Ji).length()*.5}getMidpoint(e){return e.addVectors(this.a,this.b).add(this.c).multiplyScalar(1/3)}getNormal(e){return r.getNormal(this.a,this.b,this.c,e)}getPlane(e){return e.setFromCoplanarPoints(this.a,this.b,this.c)}getBarycoord(e,t){return r.getBarycoord(e,this.a,this.b,this.c,t)}getInterpolation(e,t,i,n,o){return r.getInterpolation(e,this.a,this.b,this.c,t,i,n,o)}containsPoint(e){return r.containsPoint(e,this.a,this.b,this.c)}isFrontFacing(e){return r.isFrontFacing(this.a,this.b,this.c,e)}intersectsBox(e){return e.intersectsTriangle(this)}closestPointToPoint(e,t){let i=this.a,n=this.b,o=this.c,s,a;hn.subVectors(n,i),fn.subVectors(o,i),cc.subVectors(e,i);let c=hn.dot(cc),l=fn.dot(cc);if(c<=0&&l<=0)return t.copy(i);lc.subVectors(e,n);let d=hn.dot(lc),u=fn.dot(lc);if(d>=0&&u<=d)return t.copy(n);let h=c*u-d*l;if(h<=0&&c>=0&&d<=0)return s=c/(c-d),t.copy(i).addScaledVector(hn,s);dc.subVectors(e,o);let p=hn.dot(dc),x=fn.dot(dc);if(x>=0&&p<=x)return t.copy(o);let g=p*l-c*x;if(g<=0&&l>=0&&x<=0)return a=l/(l-x),t.copy(i).addScaledVector(fn,a);let m=d*x-p*u;if(m<=0&&u-d>=0&&p-x>=0)return su.subVectors(o,n),a=(u-d)/(u-d+(p-x)),t.copy(n).addScaledVector(su,a);let f=1/(m+g+h);return s=g*f,a=h*f,t.copy(i).addScaledVector(hn,s).addScaledVector(fn,a)}equals(e){return e.a.equals(this.a)&&e.b.equals(this.b)&&e.c.equals(this.c)}};var au=new Ee,Fr=new vs,_s=new pr,cu=new D,pn=new D,mn=new D,gn=new D,uc=new D,ys=new D,Ss=new be,bs=new be,Rs=new be,lu=new D,du=new D,uu=new D,Ms=new D,ws=new D,Xe=class extends vt{constructor(e=new Tt,t=new bi){super(),this.isMesh=!0,this.type="Mesh",this.geometry=e,this.material=t,this.updateMorphTargets()}copy(e,t){return super.copy(e,t),e.morphTargetInfluences!==void 0&&(this.morphTargetInfluences=e.morphTargetInfluences.slice()),e.morphTargetDictionary!==void 0&&(this.morphTargetDictionary=Object.assign({},e.morphTargetDictionary)),this.material=Array.isArray(e.material)?e.material.slice():e.material,this.geometry=e.geometry,this}updateMorphTargets(){let t=this.geometry.morphAttributes,i=Object.keys(t);if(i.length>0){let n=t[i[0]];if(n!==void 0){this.morphTargetInfluences=[],this.morphTargetDictionary={};for(let o=0,s=n.length;o<s;o++){let a=n[o].name||String(o);this.morphTargetInfluences.push(0),this.morphTargetDictionary[a]=o}}}}getVertexPosition(e,t){let i=this.geometry,n=i.attributes.position,o=i.morphAttributes.position,s=i.morphTargetsRelative;t.fromBufferAttribute(n,e);let a=this.morphTargetInfluences;if(o&&a){ys.set(0,0,0);for(let c=0,l=o.length;c<l;c++){let d=a[c],u=o[c];d!==0&&(uc.fromBufferAttribute(u,e),s?ys.addScaledVector(uc,d):ys.addScaledVector(uc.sub(t),d))}t.add(ys)}return t}raycast(e,t){let i=this.geometry,n=this.material,o=this.matrixWorld;n!==void 0&&(i.boundingSphere===null&&i.computeBoundingSphere(),_s.copy(i.boundingSphere),_s.applyMatrix4(o),Fr.copy(e.ray).recast(e.near),!(_s.containsPoint(Fr.origin)===!1&&(Fr.intersectSphere(_s,cu)===null||Fr.origin.distanceToSquared(cu)>(e.far-e.near)**2))&&(au.copy(o).invert(),Fr.copy(e.ray).applyMatrix4(au),!(i.boundingBox!==null&&Fr.intersectsBox(i.boundingBox)===!1)&&this._computeIntersections(e,t,Fr)))}_computeIntersections(e,t,i){let n,o=this.geometry,s=this.material,a=o.index,c=o.attributes.position,l=o.attributes.uv,d=o.attributes.uv1,u=o.attributes.normal,h=o.groups,p=o.drawRange;if(a!==null)if(Array.isArray(s))for(let x=0,g=h.length;x<g;x++){let m=h[x],f=s[m.materialIndex],S=Math.max(m.start,p.start),_=Math.min(a.count,Math.min(m.start+m.count,p.start+p.count));for(let R=S,P=_;R<P;R+=3){let w=a.getX(R),T=a.getX(R+1),C=a.getX(R+2);n=Ts(this,f,e,i,l,d,u,w,T,C),n&&(n.faceIndex=Math.floor(R/3),n.face.materialIndex=m.materialIndex,t.push(n))}}else{let x=Math.max(0,p.start),g=Math.min(a.count,p.start+p.count);for(let m=x,f=g;m<f;m+=3){let S=a.getX(m),_=a.getX(m+1),R=a.getX(m+2);n=Ts(this,s,e,i,l,d,u,S,_,R),n&&(n.faceIndex=Math.floor(m/3),t.push(n))}}else if(c!==void 0)if(Array.isArray(s))for(let x=0,g=h.length;x<g;x++){let m=h[x],f=s[m.materialIndex],S=Math.max(m.start,p.start),_=Math.min(c.count,Math.min(m.start+m.count,p.start+p.count));for(let R=S,P=_;R<P;R+=3){let w=R,T=R+1,C=R+2;n=Ts(this,f,e,i,l,d,u,w,T,C),n&&(n.faceIndex=Math.floor(R/3),n.face.materialIndex=m.materialIndex,t.push(n))}}else{let x=Math.max(0,p.start),g=Math.min(c.count,p.start+p.count);for(let m=x,f=g;m<f;m+=3){let S=m,_=m+1,R=m+2;n=Ts(this,s,e,i,l,d,u,S,_,R),n&&(n.faceIndex=Math.floor(m/3),t.push(n))}}}};function Cg(r,e,t,i,n,o,s,a){let c;if(e.side===ut?c=i.intersectTriangle(s,o,n,!0,a):c=i.intersectTriangle(n,o,s,e.side===ni,a),c===null)return null;ws.copy(a),ws.applyMatrix4(r.matrixWorld);let l=t.ray.origin.distanceTo(ws);return l<t.near||l>t.far?null:{distance:l,point:ws.clone(),object:r}}function Ts(r,e,t,i,n,o,s,a,c,l){r.getVertexPosition(a,pn),r.getVertexPosition(c,mn),r.getVertexPosition(l,gn);let d=Cg(r,e,t,i,pn,mn,gn,Ms);if(d){n&&(Ss.fromBufferAttribute(n,a),bs.fromBufferAttribute(n,c),Rs.fromBufferAttribute(n,l),d.uv=Ur.getInterpolation(Ms,pn,mn,gn,Ss,bs,Rs,new be)),o&&(Ss.fromBufferAttribute(o,a),bs.fromBufferAttribute(o,c),Rs.fromBufferAttribute(o,l),d.uv1=Ur.getInterpolation(Ms,pn,mn,gn,Ss,bs,Rs,new be)),s&&(lu.fromBufferAttribute(s,a),du.fromBufferAttribute(s,c),uu.fromBufferAttribute(s,l),d.normal=Ur.getInterpolation(Ms,pn,mn,gn,lu,du,uu,new D),d.normal.dot(i.direction)>0&&d.normal.multiplyScalar(-1));let u={a,b:c,c:l,normal:new D,materialIndex:0};Ur.getNormal(pn,mn,gn,u.normal),d.face=u}return d}var Es=class extends vt{constructor(e,t=1){super(),this.isLight=!0,this.type="Light",this.color=new Me(e),this.intensity=t}dispose(){}copy(e,t){return super.copy(e,t),this.color.copy(e.color),this.intensity=e.intensity,this}toJSON(e){let t=super.toJSON(e);return t.object.color=this.color.getHex(),t.object.intensity=this.intensity,this.groundColor!==void 0&&(t.object.groundColor=this.groundColor.getHex()),this.distance!==void 0&&(t.object.distance=this.distance),this.angle!==void 0&&(t.object.angle=this.angle),this.decay!==void 0&&(t.object.decay=this.decay),this.penumbra!==void 0&&(t.object.penumbra=this.penumbra),this.shadow!==void 0&&(t.object.shadow=this.shadow.toJSON()),t}};var ct=class r{constructor(e=0,t=0,i=0,n=1){r.prototype.isVector4=!0,this.x=e,this.y=t,this.z=i,this.w=n}get width(){return this.z}set width(e){this.z=e}get height(){return this.w}set height(e){this.w=e}set(e,t,i,n){return this.x=e,this.y=t,this.z=i,this.w=n,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this.w=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setW(e){return this.w=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;case 2:this.z=t;break;case 3:this.w=t;break;default:throw new Error("index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;case 3:return this.w;default:throw new Error("index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z,this.w)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this.w=e.w!==void 0?e.w:1,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this.w+=e.w,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this.w+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this.z=e.z+t.z,this.w=e.w+t.w,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this.z+=e.z*t,this.w+=e.w*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this.w-=e.w,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this.w-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this.z=e.z-t.z,this.w=e.w-t.w,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this.w*=e.w,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this.w*=e,this}applyMatrix4(e){let t=this.x,i=this.y,n=this.z,o=this.w,s=e.elements;return this.x=s[0]*t+s[4]*i+s[8]*n+s[12]*o,this.y=s[1]*t+s[5]*i+s[9]*n+s[13]*o,this.z=s[2]*t+s[6]*i+s[10]*n+s[14]*o,this.w=s[3]*t+s[7]*i+s[11]*n+s[15]*o,this}divideScalar(e){return this.multiplyScalar(1/e)}setAxisAngleFromQuaternion(e){this.w=2*Math.acos(e.w);let t=Math.sqrt(1-e.w*e.w);return t<1e-4?(this.x=1,this.y=0,this.z=0):(this.x=e.x/t,this.y=e.y/t,this.z=e.z/t),this}setAxisAngleFromRotationMatrix(e){let t,i,n,o,c=e.elements,l=c[0],d=c[4],u=c[8],h=c[1],p=c[5],x=c[9],g=c[2],m=c[6],f=c[10];if(Math.abs(d-h)<.01&&Math.abs(u-g)<.01&&Math.abs(x-m)<.01){if(Math.abs(d+h)<.1&&Math.abs(u+g)<.1&&Math.abs(x+m)<.1&&Math.abs(l+p+f-3)<.1)return this.set(1,0,0,0),this;t=Math.PI;let _=(l+1)/2,R=(p+1)/2,P=(f+1)/2,w=(d+h)/4,T=(u+g)/4,C=(x+m)/4;return _>R&&_>P?_<.01?(i=0,n=.707106781,o=.707106781):(i=Math.sqrt(_),n=w/i,o=T/i):R>P?R<.01?(i=.707106781,n=0,o=.707106781):(n=Math.sqrt(R),i=w/n,o=C/n):P<.01?(i=.707106781,n=.707106781,o=0):(o=Math.sqrt(P),i=T/o,n=C/o),this.set(i,n,o,t),this}let S=Math.sqrt((m-x)*(m-x)+(u-g)*(u-g)+(h-d)*(h-d));return Math.abs(S)<.001&&(S=1),this.x=(m-x)/S,this.y=(u-g)/S,this.z=(h-d)/S,this.w=Math.acos((l+p+f-1)/2),this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this.w=Math.min(this.w,e.w),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this.w=Math.max(this.w,e.w),this}clamp(e,t){return this.x=Math.max(e.x,Math.min(t.x,this.x)),this.y=Math.max(e.y,Math.min(t.y,this.y)),this.z=Math.max(e.z,Math.min(t.z,this.z)),this.w=Math.max(e.w,Math.min(t.w,this.w)),this}clampScalar(e,t){return this.x=Math.max(e,Math.min(t,this.x)),this.y=Math.max(e,Math.min(t,this.y)),this.z=Math.max(e,Math.min(t,this.z)),this.w=Math.max(e,Math.min(t,this.w)),this}clampLength(e,t){let i=this.length();return this.divideScalar(i||1).multiplyScalar(Math.max(e,Math.min(t,i)))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this.w=Math.floor(this.w),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this.w=Math.ceil(this.w),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this.w=Math.round(this.w),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this.w=Math.trunc(this.w),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this.w=-this.w,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z+this.w*e.w}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)+Math.abs(this.w)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this.z+=(e.z-this.z)*t,this.w+=(e.w-this.w)*t,this}lerpVectors(e,t,i){return this.x=e.x+(t.x-e.x)*i,this.y=e.y+(t.y-e.y)*i,this.z=e.z+(t.z-e.z)*i,this.w=e.w+(t.w-e.w)*i,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z&&e.w===this.w}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this.z=e[t+2],this.w=e[t+3],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e[t+2]=this.z,e[t+3]=this.w,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this.z=e.getZ(t),this.w=e.getW(t),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this.w=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z,yield this.w}};var hc=new D,Pg=new D,Ig=new Se,Mi=class{constructor(e=new D(1,0,0),t=0){this.isPlane=!0,this.normal=e,this.constant=t}set(e,t){return this.normal.copy(e),this.constant=t,this}setComponents(e,t,i,n){return this.normal.set(e,t,i),this.constant=n,this}setFromNormalAndCoplanarPoint(e,t){return this.normal.copy(e),this.constant=-t.dot(this.normal),this}setFromCoplanarPoints(e,t,i){let n=hc.subVectors(i,t).cross(Pg.subVectors(e,t)).normalize();return this.setFromNormalAndCoplanarPoint(n,e),this}copy(e){return this.normal.copy(e.normal),this.constant=e.constant,this}normalize(){let e=1/this.normal.length();return this.normal.multiplyScalar(e),this.constant*=e,this}negate(){return this.constant*=-1,this.normal.negate(),this}distanceToPoint(e){return this.normal.dot(e)+this.constant}distanceToSphere(e){return this.distanceToPoint(e.center)-e.radius}projectPoint(e,t){return t.copy(e).addScaledVector(this.normal,-this.distanceToPoint(e))}intersectLine(e,t){let i=e.delta(hc),n=this.normal.dot(i);if(n===0)return this.distanceToPoint(e.start)===0?t.copy(e.start):null;let o=-(e.start.dot(this.normal)+this.constant)/n;return o<0||o>1?null:t.copy(e.start).addScaledVector(i,o)}intersectsLine(e){let t=this.distanceToPoint(e.start),i=this.distanceToPoint(e.end);return t<0&&i>0||i<0&&t>0}intersectsBox(e){return e.intersectsPlane(this)}intersectsSphere(e){return e.intersectsPlane(this)}coplanarPoint(e){return e.copy(this.normal).multiplyScalar(-this.constant)}applyMatrix4(e,t){let i=t||Ig.getNormalMatrix(e),n=this.coplanarPoint(hc).applyMatrix4(e),o=this.normal.applyMatrix3(i).normalize();return this.constant=-n.dot(o),this}translate(e){return this.constant-=e.dot(this.normal),this}equals(e){return e.normal.equals(this.normal)&&e.constant===this.constant}clone(){return new this.constructor().copy(this)}};var Or=new pr,As=new D,_r=class{constructor(e=new Mi,t=new Mi,i=new Mi,n=new Mi,o=new Mi,s=new Mi){this.planes=[e,t,i,n,o,s]}set(e,t,i,n,o,s){let a=this.planes;return a[0].copy(e),a[1].copy(t),a[2].copy(i),a[3].copy(n),a[4].copy(o),a[5].copy(s),this}copy(e){let t=this.planes;for(let i=0;i<6;i++)t[i].copy(e.planes[i]);return this}setFromProjectionMatrix(e,t=ti){let i=this.planes,n=e.elements,o=n[0],s=n[1],a=n[2],c=n[3],l=n[4],d=n[5],u=n[6],h=n[7],p=n[8],x=n[9],g=n[10],m=n[11],f=n[12],S=n[13],_=n[14],R=n[15];if(i[0].setComponents(c-o,h-l,m-p,R-f).normalize(),i[1].setComponents(c+o,h+l,m+p,R+f).normalize(),i[2].setComponents(c+s,h+d,m+x,R+S).normalize(),i[3].setComponents(c-s,h-d,m-x,R-S).normalize(),i[4].setComponents(c-a,h-u,m-g,R-_).normalize(),t===ti)i[5].setComponents(c+a,h+u,m+g,R+_).normalize();else if(t===Lr)i[5].setComponents(a,u,g,_).normalize();else throw new Error("THREE.Frustum.setFromProjectionMatrix(): Invalid coordinate system: "+t);return this}intersectsObject(e){if(e.boundingSphere!==void 0)e.boundingSphere===null&&e.computeBoundingSphere(),Or.copy(e.boundingSphere).applyMatrix4(e.matrixWorld);else{let t=e.geometry;t.boundingSphere===null&&t.computeBoundingSphere(),Or.copy(t.boundingSphere).applyMatrix4(e.matrixWorld)}return this.intersectsSphere(Or)}intersectsSprite(e){return Or.center.set(0,0,0),Or.radius=.7071067811865476,Or.applyMatrix4(e.matrixWorld),this.intersectsSphere(Or)}intersectsSphere(e){let t=this.planes,i=e.center,n=-e.radius;for(let o=0;o<6;o++)if(t[o].distanceToPoint(i)<n)return!1;return!0}intersectsBox(e){let t=this.planes;for(let i=0;i<6;i++){let n=t[i];if(As.x=n.normal.x>0?e.max.x:e.min.x,As.y=n.normal.y>0?e.max.y:e.min.y,As.z=n.normal.z>0?e.max.z:e.min.z,n.distanceToPoint(As)<0)return!1}return!0}containsPoint(e){let t=this.planes;for(let i=0;i<6;i++)if(t[i].distanceToPoint(e)<0)return!1;return!0}clone(){return new this.constructor().copy(this)}};var fc=new Ee,hu=new D,fu=new D,Cs=class{constructor(e){this.camera=e,this.bias=0,this.normalBias=0,this.radius=1,this.blurSamples=8,this.mapSize=new be(512,512),this.map=null,this.mapPass=null,this.matrix=new Ee,this.autoUpdate=!0,this.needsUpdate=!1,this._frustum=new _r,this._frameExtents=new be(1,1),this._viewportCount=1,this._viewports=[new ct(0,0,1,1)]}getViewportCount(){return this._viewportCount}getFrustum(){return this._frustum}updateMatrices(e){let t=this.camera,i=this.matrix;hu.setFromMatrixPosition(e.matrixWorld),t.position.copy(hu),fu.setFromMatrixPosition(e.target.matrixWorld),t.lookAt(fu),t.updateMatrixWorld(),fc.multiplyMatrices(t.projectionMatrix,t.matrixWorldInverse),this._frustum.setFromProjectionMatrix(fc),i.set(.5,0,0,.5,0,.5,0,.5,0,0,.5,.5,0,0,0,1),i.multiply(fc)}getViewport(e){return this._viewports[e]}getFrameExtents(){return this._frameExtents}dispose(){this.map&&this.map.dispose(),this.mapPass&&this.mapPass.dispose()}copy(e){return this.camera=e.camera.clone(),this.bias=e.bias,this.radius=e.radius,this.mapSize.copy(e.mapSize),this}clone(){return new this.constructor().copy(this)}toJSON(){let e={};return this.bias!==0&&(e.bias=this.bias),this.normalBias!==0&&(e.normalBias=this.normalBias),this.radius!==1&&(e.radius=this.radius),(this.mapSize.x!==512||this.mapSize.y!==512)&&(e.mapSize=this.mapSize.toArray()),e.camera=this.camera.toJSON(!1).object,delete e.camera.matrix,e}};var Ps=class extends Cs{constructor(){super(new un(-5,5,5,-5,.5,500)),this.isDirectionalLightShadow=!0}};var ho=class extends Es{constructor(e,t){super(e,t),this.isDirectionalLight=!0,this.type="DirectionalLight",this.position.copy(vt.DEFAULT_UP),this.updateMatrix(),this.target=new vt,this.shadow=new Ps}dispose(){this.shadow.dispose()}copy(e){return super.copy(e),this.target=e.target.clone(),this.shadow=e.shadow.clone(),this}};var Nr=class extends vt{constructor(){super(),this.isScene=!0,this.type="Scene",this.background=null,this.environment=null,this.fog=null,this.backgroundBlurriness=0,this.backgroundIntensity=1,this.backgroundRotation=new ii,this.environmentIntensity=1,this.environmentRotation=new ii,this.overrideMaterial=null,typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}copy(e,t){return super.copy(e,t),e.background!==null&&(this.background=e.background.clone()),e.environment!==null&&(this.environment=e.environment.clone()),e.fog!==null&&(this.fog=e.fog.clone()),this.backgroundBlurriness=e.backgroundBlurriness,this.backgroundIntensity=e.backgroundIntensity,this.backgroundRotation.copy(e.backgroundRotation),this.environmentIntensity=e.environmentIntensity,this.environmentRotation.copy(e.environmentRotation),e.overrideMaterial!==null&&(this.overrideMaterial=e.overrideMaterial.clone()),this.matrixAutoUpdate=e.matrixAutoUpdate,this}toJSON(e){let t=super.toJSON(e);return this.fog!==null&&(t.object.fog=this.fog.toJSON()),this.backgroundBlurriness>0&&(t.object.backgroundBlurriness=this.backgroundBlurriness),this.backgroundIntensity!==1&&(t.object.backgroundIntensity=this.backgroundIntensity),t.object.backgroundRotation=this.backgroundRotation.toArray(),this.environmentIntensity!==1&&(t.object.environmentIntensity=this.environmentIntensity),t.object.environmentRotation=this.environmentRotation.toArray(),t}};var Dg=0,xn=class{constructor(e=null){this.isSource=!0,Object.defineProperty(this,"id",{value:Dg++}),this.uuid=Di(),this.data=e,this.dataReady=!0,this.version=0}set needsUpdate(e){e===!0&&this.version++}toJSON(e){let t=e===void 0||typeof e=="string";if(!t&&e.images[this.uuid]!==void 0)return e.images[this.uuid];let i={uuid:this.uuid,url:""},n=this.data;if(n!==null){let o;if(Array.isArray(n)){o=[];for(let s=0,a=n.length;s<a;s++)n[s].isDataTexture?o.push(pc(n[s].image)):o.push(pc(n[s]))}else o=pc(n);i.url=o}return t||(e.images[this.uuid]=i),i}};function pc(r){return r.data?{data:Array.from(r.data),width:r.width,height:r.height,type:r.data.constructor.name}:(console.warn("THREE.Texture: Unable to serialize Texture."),{})}var Lg=0,St=class r extends Qt{constructor(e=r.DEFAULT_IMAGE,t=r.DEFAULT_MAPPING,i=Yt,n=Yt,o=ht,s=_i,a=Mt,c=Ut,l=r.DEFAULT_ANISOTROPY,d=hi){super(),this.isTexture=!0,Object.defineProperty(this,"id",{value:Lg++}),this.uuid=Di(),this.name="",this.source=new xn(e),this.mipmaps=[],this.mapping=t,this.channel=0,this.wrapS=i,this.wrapT=n,this.magFilter=o,this.minFilter=s,this.anisotropy=l,this.format=a,this.internalFormat=null,this.type=c,this.offset=new be(0,0),this.repeat=new be(1,1),this.center=new be(0,0),this.rotation=0,this.matrixAutoUpdate=!0,this.matrix=new Se,this.generateMipmaps=!0,this.premultiplyAlpha=!1,this.flipY=!0,this.unpackAlignment=4,this.colorSpace=d,this.userData={},this.version=0,this.onUpdate=null,this.isRenderTargetTexture=!1,this.pmremVersion=0}get image(){return this.source.data}set image(e=null){this.source.data=e}updateMatrix(){this.matrix.setUvTransform(this.offset.x,this.offset.y,this.repeat.x,this.repeat.y,this.rotation,this.center.x,this.center.y)}clone(){return new this.constructor().copy(this)}copy(e){return this.name=e.name,this.source=e.source,this.mipmaps=e.mipmaps.slice(0),this.mapping=e.mapping,this.channel=e.channel,this.wrapS=e.wrapS,this.wrapT=e.wrapT,this.magFilter=e.magFilter,this.minFilter=e.minFilter,this.anisotropy=e.anisotropy,this.format=e.format,this.internalFormat=e.internalFormat,this.type=e.type,this.offset.copy(e.offset),this.repeat.copy(e.repeat),this.center.copy(e.center),this.rotation=e.rotation,this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrix.copy(e.matrix),this.generateMipmaps=e.generateMipmaps,this.premultiplyAlpha=e.premultiplyAlpha,this.flipY=e.flipY,this.unpackAlignment=e.unpackAlignment,this.colorSpace=e.colorSpace,this.userData=JSON.parse(JSON.stringify(e.userData)),this.needsUpdate=!0,this}toJSON(e){let t=e===void 0||typeof e=="string";if(!t&&e.textures[this.uuid]!==void 0)return e.textures[this.uuid];let i={metadata:{version:4.6,type:"Texture",generator:"Texture.toJSON"},uuid:this.uuid,name:this.name,image:this.source.toJSON(e).uuid,mapping:this.mapping,channel:this.channel,repeat:[this.repeat.x,this.repeat.y],offset:[this.offset.x,this.offset.y],center:[this.center.x,this.center.y],rotation:this.rotation,wrap:[this.wrapS,this.wrapT],format:this.format,internalFormat:this.internalFormat,type:this.type,colorSpace:this.colorSpace,minFilter:this.minFilter,magFilter:this.magFilter,anisotropy:this.anisotropy,flipY:this.flipY,generateMipmaps:this.generateMipmaps,premultiplyAlpha:this.premultiplyAlpha,unpackAlignment:this.unpackAlignment};return Object.keys(this.userData).length>0&&(i.userData=this.userData),t||(e.textures[this.uuid]=i),i}dispose(){this.dispatchEvent({type:"dispose"})}transformUv(e){if(this.mapping!==Ra)return e;if(e.applyMatrix3(this.matrix),e.x<0||e.x>1)switch(this.wrapS){case ro:e.x=e.x-Math.floor(e.x);break;case Yt:e.x=e.x<0?0:1;break;case no:Math.abs(Math.floor(e.x)%2)===1?e.x=Math.ceil(e.x)-e.x:e.x=e.x-Math.floor(e.x);break}if(e.y<0||e.y>1)switch(this.wrapT){case ro:e.y=e.y-Math.floor(e.y);break;case Yt:e.y=e.y<0?0:1;break;case no:Math.abs(Math.floor(e.y)%2)===1?e.y=Math.ceil(e.y)-e.y:e.y=e.y-Math.floor(e.y);break}return this.flipY&&(e.y=1-e.y),e}set needsUpdate(e){e===!0&&(this.version++,this.source.needsUpdate=!0)}set needsPMREMUpdate(e){e===!0&&this.pmremVersion++}};St.DEFAULT_IMAGE=null;St.DEFAULT_MAPPING=Ra;St.DEFAULT_ANISOTROPY=1;var vn=class extends St{constructor(e=null,t=1,i=1,n,o,s,a,c,l=nt,d=nt,u,h){super(null,s,a,c,l,d,n,o,u,h),this.isDataTexture=!0,this.image={data:e,width:t,height:i},this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}};var Is=class extends Qt{constructor(e=1,t=1,i={}){super(),this.isRenderTarget=!0,this.width=e,this.height=t,this.depth=1,this.scissor=new ct(0,0,e,t),this.scissorTest=!1,this.viewport=new ct(0,0,e,t);let n={width:e,height:t,depth:1};i=Object.assign({generateMipmaps:!1,internalFormat:null,minFilter:ht,depthBuffer:!0,stencilBuffer:!1,resolveDepthBuffer:!0,resolveStencilBuffer:!0,depthTexture:null,samples:0,count:1},i);let o=new St(n,i.mapping,i.wrapS,i.wrapT,i.magFilter,i.minFilter,i.format,i.type,i.anisotropy,i.colorSpace);o.flipY=!1,o.generateMipmaps=i.generateMipmaps,o.internalFormat=i.internalFormat,this.textures=[];let s=i.count;for(let a=0;a<s;a++)this.textures[a]=o.clone(),this.textures[a].isRenderTargetTexture=!0;this.depthBuffer=i.depthBuffer,this.stencilBuffer=i.stencilBuffer,this.resolveDepthBuffer=i.resolveDepthBuffer,this.resolveStencilBuffer=i.resolveStencilBuffer,this.depthTexture=i.depthTexture,this.samples=i.samples}get texture(){return this.textures[0]}set texture(e){this.textures[0]=e}setSize(e,t,i=1){if(this.width!==e||this.height!==t||this.depth!==i){this.width=e,this.height=t,this.depth=i;for(let n=0,o=this.textures.length;n<o;n++)this.textures[n].image.width=e,this.textures[n].image.height=t,this.textures[n].image.depth=i;this.dispose()}this.viewport.set(0,0,e,t),this.scissor.set(0,0,e,t)}clone(){return new this.constructor().copy(this)}copy(e){this.width=e.width,this.height=e.height,this.depth=e.depth,this.scissor.copy(e.scissor),this.scissorTest=e.scissorTest,this.viewport.copy(e.viewport),this.textures.length=0;for(let i=0,n=e.textures.length;i<n;i++)this.textures[i]=e.textures[i].clone(),this.textures[i].isRenderTargetTexture=!0;let t=Object.assign({},e.texture.image);return this.texture.source=new xn(t),this.depthBuffer=e.depthBuffer,this.stencilBuffer=e.stencilBuffer,this.resolveDepthBuffer=e.resolveDepthBuffer,this.resolveStencilBuffer=e.resolveStencilBuffer,e.depthTexture!==null&&(this.depthTexture=e.depthTexture.clone()),this.samples=e.samples,this}dispose(){this.dispatchEvent({type:"dispose"})}};var Ot=class extends Is{constructor(e=1,t=1,i={}){super(e,t,i),this.isWebGLRenderTarget=!0}};function Ds(){let r=null,e=!1,t=null,i=null;function n(o,s){t(o,s),i=r.requestAnimationFrame(n)}return{start:function(){e!==!0&&t!==null&&(i=r.requestAnimationFrame(n),e=!0)},stop:function(){r.cancelAnimationFrame(i),e=!1},setAnimationLoop:function(o){t=o},setContext:function(o){r=o}}}function pu(r){let e=new WeakMap;function t(a,c){let l=a.array,d=a.usage,u=l.byteLength,h=r.createBuffer();r.bindBuffer(c,h),r.bufferData(c,l,d),a.onUploadCallback();let p;if(l instanceof Float32Array)p=r.FLOAT;else if(l instanceof Uint16Array)a.isFloat16BufferAttribute?p=r.HALF_FLOAT:p=r.UNSIGNED_SHORT;else if(l instanceof Int16Array)p=r.SHORT;else if(l instanceof Uint32Array)p=r.UNSIGNED_INT;else if(l instanceof Int32Array)p=r.INT;else if(l instanceof Int8Array)p=r.BYTE;else if(l instanceof Uint8Array)p=r.UNSIGNED_BYTE;else if(l instanceof Uint8ClampedArray)p=r.UNSIGNED_BYTE;else throw new Error("THREE.WebGLAttributes: Unsupported buffer data format: "+l);return{buffer:h,type:p,bytesPerElement:l.BYTES_PER_ELEMENT,version:a.version,size:u}}function i(a,c,l){let d=c.array,u=c._updateRange,h=c.updateRanges;if(r.bindBuffer(l,a),u.count===-1&&h.length===0&&r.bufferSubData(l,0,d),h.length!==0){for(let p=0,x=h.length;p<x;p++){let g=h[p];r.bufferSubData(l,g.start*d.BYTES_PER_ELEMENT,d,g.start,g.count)}c.clearUpdateRanges()}u.count!==-1&&(r.bufferSubData(l,u.offset*d.BYTES_PER_ELEMENT,d,u.offset,u.count),u.count=-1),c.onUploadCallback()}function n(a){return a.isInterleavedBufferAttribute&&(a=a.data),e.get(a)}function o(a){a.isInterleavedBufferAttribute&&(a=a.data);let c=e.get(a);c&&(r.deleteBuffer(c.buffer),e.delete(a))}function s(a,c){if(a.isGLBufferAttribute){let d=e.get(a);(!d||d.version<a.version)&&e.set(a,{buffer:a.buffer,type:a.type,bytesPerElement:a.elementSize,version:a.version});return}a.isInterleavedBufferAttribute&&(a=a.data);let l=e.get(a);if(l===void 0)e.set(a,t(a,c));else if(l.version<a.version){if(l.size!==a.array.byteLength)throw new Error("THREE.WebGLAttributes: The size of the buffer attribute's array buffer does not match the original size. Resizing buffer attributes is not supported.");i(l.buffer,a,c),l.version=a.version}}return{get:n,remove:o,update:s}}var yr=class r extends Tt{constructor(e=1,t=1,i=1,n=1,o=1,s=1){super(),this.type="BoxGeometry",this.parameters={width:e,height:t,depth:i,widthSegments:n,heightSegments:o,depthSegments:s};let a=this;n=Math.floor(n),o=Math.floor(o),s=Math.floor(s);let c=[],l=[],d=[],u=[],h=0,p=0;x("z","y","x",-1,-1,i,t,e,s,o,0),x("z","y","x",1,-1,i,t,-e,s,o,1),x("x","z","y",1,1,e,i,t,n,s,2),x("x","z","y",1,-1,e,i,-t,n,s,3),x("x","y","z",1,-1,e,t,i,n,o,4),x("x","y","z",-1,-1,e,t,-i,n,o,5),this.setIndex(c),this.setAttribute("position",new at(l,3)),this.setAttribute("normal",new at(d,3)),this.setAttribute("uv",new at(u,2));function x(g,m,f,S,_,R,P,w,T,C,y){let v=R/T,I=P/C,F=R/2,E=P/2,N=w/2,W=T+1,X=C+1,te=0,V=0,J=new D;for(let Q=0;Q<X;Q++){let me=Q*I-E;for(let Ne=0;Ne<W;Ne++){let tt=Ne*v-F;J[g]=tt*S,J[m]=me*_,J[f]=N,l.push(J.x,J.y,J.z),J[g]=0,J[m]=0,J[f]=w>0?1:-1,d.push(J.x,J.y,J.z),u.push(Ne/T),u.push(1-Q/C),te+=1}}for(let Q=0;Q<C;Q++)for(let me=0;me<T;me++){let Ne=h+me+W*Q,tt=h+me+W*(Q+1),H=h+(me+1)+W*(Q+1),ee=h+(me+1)+W*Q;c.push(Ne,tt,ee),c.push(tt,H,ee),V+=6}a.addGroup(p,V,y),p+=V,h+=te}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new r(e.width,e.height,e.depth,e.widthSegments,e.heightSegments,e.depthSegments)}};var mu=`
#ifdef USE_ALPHAHASH

	if ( diffuseColor.a < getAlphaHashThreshold( vPosition ) ) discard;

#endif
`;var gu=`
#ifdef USE_ALPHAHASH

	/**
	 * See: https://casual-effects.com/research/Wyman2017Hashed/index.html
	 */

	const float ALPHA_HASH_SCALE = 0.05; // Derived from trials only, and may be changed.

	float hash2D( vec2 value ) {

		return fract( 1.0e4 * sin( 17.0 * value.x + 0.1 * value.y ) * ( 0.1 + abs( sin( 13.0 * value.y + value.x ) ) ) );

	}

	float hash3D( vec3 value ) {

		return hash2D( vec2( hash2D( value.xy ), value.z ) );

	}

	float getAlphaHashThreshold( vec3 position ) {

		// Find the discretized derivatives of our coordinates
		float maxDeriv = max(
			length( dFdx( position.xyz ) ),
			length( dFdy( position.xyz ) )
		);
		float pixScale = 1.0 / ( ALPHA_HASH_SCALE * maxDeriv );

		// Find two nearest log-discretized noise scales
		vec2 pixScales = vec2(
			exp2( floor( log2( pixScale ) ) ),
			exp2( ceil( log2( pixScale ) ) )
		);

		// Compute alpha thresholds at our two noise scales
		vec2 alpha = vec2(
			hash3D( floor( pixScales.x * position.xyz ) ),
			hash3D( floor( pixScales.y * position.xyz ) )
		);

		// Factor to interpolate lerp with
		float lerpFactor = fract( log2( pixScale ) );

		// Interpolate alpha threshold from noise at two scales
		float x = ( 1.0 - lerpFactor ) * alpha.x + lerpFactor * alpha.y;

		// Pass into CDF to compute uniformly distrib threshold
		float a = min( lerpFactor, 1.0 - lerpFactor );
		vec3 cases = vec3(
			x * x / ( 2.0 * a * ( 1.0 - a ) ),
			( x - 0.5 * a ) / ( 1.0 - a ),
			1.0 - ( ( 1.0 - x ) * ( 1.0 - x ) / ( 2.0 * a * ( 1.0 - a ) ) )
		);

		// Find our final, uniformly distributed alpha threshold (\u03B1\u03C4)
		float threshold = ( x < ( 1.0 - a ) )
			? ( ( x < a ) ? cases.x : cases.y )
			: cases.z;

		// Avoids \u03B1\u03C4 == 0. Could also do \u03B1\u03C4 =1-\u03B1\u03C4
		return clamp( threshold , 1.0e-6, 1.0 );

	}

#endif
`;var xu=`
#ifdef USE_ALPHAMAP

	diffuseColor.a *= texture2D( alphaMap, vAlphaMapUv ).g;

#endif
`;var vu=`
#ifdef USE_ALPHAMAP

	uniform sampler2D alphaMap;

#endif
`;var _u=`
#ifdef USE_ALPHATEST

	#ifdef ALPHA_TO_COVERAGE

	diffuseColor.a = smoothstep( alphaTest, alphaTest + fwidth( diffuseColor.a ), diffuseColor.a );
	if ( diffuseColor.a == 0.0 ) discard;

	#else

	if ( diffuseColor.a < alphaTest ) discard;

	#endif

#endif
`;var yu=`
#ifdef USE_ALPHATEST
	uniform float alphaTest;
#endif
`;var Su=`
#ifdef USE_AOMAP

	// reads channel R, compatible with a combined OcclusionRoughnessMetallic (RGB) texture
	float ambientOcclusion = ( texture2D( aoMap, vAoMapUv ).r - 1.0 ) * aoMapIntensity + 1.0;

	reflectedLight.indirectDiffuse *= ambientOcclusion;

	#if defined( USE_CLEARCOAT ) 
		clearcoatSpecularIndirect *= ambientOcclusion;
	#endif

	#if defined( USE_SHEEN ) 
		sheenSpecularIndirect *= ambientOcclusion;
	#endif

	#if defined( USE_ENVMAP ) && defined( STANDARD )

		float dotNV = saturate( dot( geometryNormal, geometryViewDir ) );

		reflectedLight.indirectSpecular *= computeSpecularOcclusion( dotNV, ambientOcclusion, material.roughness );

	#endif

#endif
`;var bu=`
#ifdef USE_AOMAP

	uniform sampler2D aoMap;
	uniform float aoMapIntensity;

#endif
`;var Ru=`
#ifdef USE_BATCHING
	attribute float batchId;
	uniform highp sampler2D batchingTexture;
	mat4 getBatchingMatrix( const in float i ) {

		int size = textureSize( batchingTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( batchingTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( batchingTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( batchingTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( batchingTexture, ivec2( x + 3, y ), 0 );
		return mat4( v1, v2, v3, v4 );

	}
#endif
`;var Mu=`
#ifdef USE_BATCHING
	mat4 batchingMatrix = getBatchingMatrix( batchId );
#endif
`;var wu=`
vec3 transformed = vec3( position );

#ifdef USE_ALPHAHASH

	vPosition = vec3( position );

#endif
`;var Tu=`
vec3 objectNormal = vec3( normal );

#ifdef USE_TANGENT

	vec3 objectTangent = vec3( tangent.xyz );

#endif
`;var Eu=`

float G_BlinnPhong_Implicit( /* const in float dotNL, const in float dotNV */ ) {

	// geometry term is (n dot l)(n dot v) / 4(n dot l)(n dot v)
	return 0.25;

}

float D_BlinnPhong( const in float shininess, const in float dotNH ) {

	return RECIPROCAL_PI * ( shininess * 0.5 + 1.0 ) * pow( dotNH, shininess );

}

vec3 BRDF_BlinnPhong( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in vec3 specularColor, const in float shininess ) {

	vec3 halfDir = normalize( lightDir + viewDir );

	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );

	vec3 F = F_Schlick( specularColor, 1.0, dotVH );

	float G = G_BlinnPhong_Implicit( /* dotNL, dotNV */ );

	float D = D_BlinnPhong( shininess, dotNH );

	return F * ( G * D );

} // validated

`;var Au=`

#ifdef USE_IRIDESCENCE

	// XYZ to linear-sRGB color space
	const mat3 XYZ_TO_REC709 = mat3(
		 3.2404542, -0.9692660,  0.0556434,
		-1.5371385,  1.8760108, -0.2040259,
		-0.4985314,  0.0415560,  1.0572252
	);

	// Assume air interface for top
	// Note: We don't handle the case fresnel0 == 1
	vec3 Fresnel0ToIor( vec3 fresnel0 ) {

		vec3 sqrtF0 = sqrt( fresnel0 );
		return ( vec3( 1.0 ) + sqrtF0 ) / ( vec3( 1.0 ) - sqrtF0 );

	}

	// Conversion FO/IOR
	vec3 IorToFresnel0( vec3 transmittedIor, float incidentIor ) {

		return pow2( ( transmittedIor - vec3( incidentIor ) ) / ( transmittedIor + vec3( incidentIor ) ) );

	}

	// ior is a value between 1.0 and 3.0. 1.0 is air interface
	float IorToFresnel0( float transmittedIor, float incidentIor ) {

		return pow2( ( transmittedIor - incidentIor ) / ( transmittedIor + incidentIor ));

	}

	// Fresnel equations for dielectric/dielectric interfaces.
	// Ref: https://belcour.github.io/blog/research/2017/05/01/brdf-thin-film.html
	// Evaluation XYZ sensitivity curves in Fourier space
	vec3 evalSensitivity( float OPD, vec3 shift ) {

		float phase = 2.0 * PI * OPD * 1.0e-9;
		vec3 val = vec3( 5.4856e-13, 4.4201e-13, 5.2481e-13 );
		vec3 pos = vec3( 1.6810e+06, 1.7953e+06, 2.2084e+06 );
		vec3 var = vec3( 4.3278e+09, 9.3046e+09, 6.6121e+09 );

		vec3 xyz = val * sqrt( 2.0 * PI * var ) * cos( pos * phase + shift ) * exp( - pow2( phase ) * var );
		xyz.x += 9.7470e-14 * sqrt( 2.0 * PI * 4.5282e+09 ) * cos( 2.2399e+06 * phase + shift[ 0 ] ) * exp( - 4.5282e+09 * pow2( phase ) );
		xyz /= 1.0685e-7;

		vec3 rgb = XYZ_TO_REC709 * xyz;
		return rgb;

	}

	vec3 evalIridescence( float outsideIOR, float eta2, float cosTheta1, float thinFilmThickness, vec3 baseF0 ) {

		vec3 I;

		// Force iridescenceIOR -> outsideIOR when thinFilmThickness -> 0.0
		float iridescenceIOR = mix( outsideIOR, eta2, smoothstep( 0.0, 0.03, thinFilmThickness ) );
		// Evaluate the cosTheta on the base layer (Snell law)
		float sinTheta2Sq = pow2( outsideIOR / iridescenceIOR ) * ( 1.0 - pow2( cosTheta1 ) );

		// Handle TIR:
		float cosTheta2Sq = 1.0 - sinTheta2Sq;
		if ( cosTheta2Sq < 0.0 ) {

			return vec3( 1.0 );

		}

		float cosTheta2 = sqrt( cosTheta2Sq );

		// First interface
		float R0 = IorToFresnel0( iridescenceIOR, outsideIOR );
		float R12 = F_Schlick( R0, 1.0, cosTheta1 );
		float T121 = 1.0 - R12;
		float phi12 = 0.0;
		if ( iridescenceIOR < outsideIOR ) phi12 = PI;
		float phi21 = PI - phi12;

		// Second interface
		vec3 baseIOR = Fresnel0ToIor( clamp( baseF0, 0.0, 0.9999 ) ); // guard against 1.0
		vec3 R1 = IorToFresnel0( baseIOR, iridescenceIOR );
		vec3 R23 = F_Schlick( R1, 1.0, cosTheta2 );
		vec3 phi23 = vec3( 0.0 );
		if ( baseIOR[ 0 ] < iridescenceIOR ) phi23[ 0 ] = PI;
		if ( baseIOR[ 1 ] < iridescenceIOR ) phi23[ 1 ] = PI;
		if ( baseIOR[ 2 ] < iridescenceIOR ) phi23[ 2 ] = PI;

		// Phase shift
		float OPD = 2.0 * iridescenceIOR * thinFilmThickness * cosTheta2;
		vec3 phi = vec3( phi21 ) + phi23;

		// Compound terms
		vec3 R123 = clamp( R12 * R23, 1e-5, 0.9999 );
		vec3 r123 = sqrt( R123 );
		vec3 Rs = pow2( T121 ) * R23 / ( vec3( 1.0 ) - R123 );

		// Reflectance term for m = 0 (DC term amplitude)
		vec3 C0 = R12 + Rs;
		I = C0;

		// Reflectance term for m > 0 (pairs of diracs)
		vec3 Cm = Rs - T121;
		for ( int m = 1; m <= 2; ++ m ) {

			Cm *= r123;
			vec3 Sm = 2.0 * evalSensitivity( float( m ) * OPD, float( m ) * phi );
			I += Cm * Sm;

		}

		// Since out of gamut colors might be produced, negative color values are clamped to 0.
		return max( I, vec3( 0.0 ) );

	}

#endif

`;var Cu=`
#ifdef USE_BUMPMAP

	uniform sampler2D bumpMap;
	uniform float bumpScale;

	// Bump Mapping Unparametrized Surfaces on the GPU by Morten S. Mikkelsen
	// https://mmikk.github.io/papers3d/mm_sfgrad_bump.pdf

	// Evaluate the derivative of the height w.r.t. screen-space using forward differencing (listing 2)

	vec2 dHdxy_fwd() {

		vec2 dSTdx = dFdx( vBumpMapUv );
		vec2 dSTdy = dFdy( vBumpMapUv );

		float Hll = bumpScale * texture2D( bumpMap, vBumpMapUv ).x;
		float dBx = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdx ).x - Hll;
		float dBy = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdy ).x - Hll;

		return vec2( dBx, dBy );

	}

	vec3 perturbNormalArb( vec3 surf_pos, vec3 surf_norm, vec2 dHdxy, float faceDirection ) {

		// normalize is done to ensure that the bump map looks the same regardless of the texture's scale
		vec3 vSigmaX = normalize( dFdx( surf_pos.xyz ) );
		vec3 vSigmaY = normalize( dFdy( surf_pos.xyz ) );
		vec3 vN = surf_norm; // normalized

		vec3 R1 = cross( vSigmaY, vN );
		vec3 R2 = cross( vN, vSigmaX );

		float fDet = dot( vSigmaX, R1 ) * faceDirection;

		vec3 vGrad = sign( fDet ) * ( dHdxy.x * R1 + dHdxy.y * R2 );
		return normalize( abs( fDet ) * surf_norm - vGrad );

	}

#endif
`;var Pu=`
#if NUM_CLIPPING_PLANES > 0

	vec4 plane;

	#ifdef ALPHA_TO_COVERAGE

		float distanceToPlane, distanceGradient;
		float clipOpacity = 1.0;

		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {

			plane = clippingPlanes[ i ];
			distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
			distanceGradient = fwidth( distanceToPlane ) / 2.0;
			clipOpacity *= smoothstep( - distanceGradient, distanceGradient, distanceToPlane );

			if ( clipOpacity == 0.0 ) discard;

		}
		#pragma unroll_loop_end

		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES

			float unionClipOpacity = 1.0;

			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {

				plane = clippingPlanes[ i ];
				distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
				distanceGradient = fwidth( distanceToPlane ) / 2.0;
				unionClipOpacity *= 1.0 - smoothstep( - distanceGradient, distanceGradient, distanceToPlane );

			}
			#pragma unroll_loop_end

			clipOpacity *= 1.0 - unionClipOpacity;

		#endif

		diffuseColor.a *= clipOpacity;

		if ( diffuseColor.a == 0.0 ) discard;

	#else

		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {

			plane = clippingPlanes[ i ];
			if ( dot( vClipPosition, plane.xyz ) > plane.w ) discard;

		}
		#pragma unroll_loop_end

		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES

			bool clipped = true;

			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {

				plane = clippingPlanes[ i ];
				clipped = ( dot( vClipPosition, plane.xyz ) > plane.w ) && clipped;

			}
			#pragma unroll_loop_end

			if ( clipped ) discard;

		#endif

	#endif

#endif
`;var Iu=`
#if NUM_CLIPPING_PLANES > 0

	varying vec3 vClipPosition;

	uniform vec4 clippingPlanes[ NUM_CLIPPING_PLANES ];

#endif
`;var Du=`
#if NUM_CLIPPING_PLANES > 0

	varying vec3 vClipPosition;

#endif
`;var Lu=`
#if NUM_CLIPPING_PLANES > 0

	vClipPosition = - mvPosition.xyz;

#endif
`;var Uu=`
#if defined( USE_COLOR_ALPHA )

	diffuseColor *= vColor;

#elif defined( USE_COLOR )

	diffuseColor.rgb *= vColor;

#endif
`;var Fu=`
#if defined( USE_COLOR_ALPHA )

	varying vec4 vColor;

#elif defined( USE_COLOR )

	varying vec3 vColor;

#endif
`;var Ou=`
#if defined( USE_COLOR_ALPHA )

	varying vec4 vColor;

#elif defined( USE_COLOR ) || defined( USE_INSTANCING_COLOR )

	varying vec3 vColor;

#endif
`;var Nu=`
#if defined( USE_COLOR_ALPHA )

	vColor = vec4( 1.0 );

#elif defined( USE_COLOR ) || defined( USE_INSTANCING_COLOR )

	vColor = vec3( 1.0 );

#endif

#ifdef USE_COLOR

	vColor *= color;

#endif

#ifdef USE_INSTANCING_COLOR

	vColor.xyz *= instanceColor.xyz;

#endif
`;var Bu=`
#define PI 3.141592653589793
#define PI2 6.283185307179586
#define PI_HALF 1.5707963267948966
#define RECIPROCAL_PI 0.3183098861837907
#define RECIPROCAL_PI2 0.15915494309189535
#define EPSILON 1e-6

#ifndef saturate
// <tonemapping_pars_fragment> may have defined saturate() already
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif
#define whiteComplement( a ) ( 1.0 - saturate( a ) )

float pow2( const in float x ) { return x*x; }
vec3 pow2( const in vec3 x ) { return x*x; }
float pow3( const in float x ) { return x*x*x; }
float pow4( const in float x ) { float x2 = x*x; return x2*x2; }
float max3( const in vec3 v ) { return max( max( v.x, v.y ), v.z ); }
float average( const in vec3 v ) { return dot( v, vec3( 0.3333333 ) ); }

// expects values in the range of [0,1]x[0,1], returns values in the [0,1] range.
// do not collapse into a single function per: http://byteblacksmith.com/improvements-to-the-canonical-one-liner-glsl-rand-for-opengl-es-2-0/
highp float rand( const in vec2 uv ) {

	const highp float a = 12.9898, b = 78.233, c = 43758.5453;
	highp float dt = dot( uv.xy, vec2( a,b ) ), sn = mod( dt, PI );

	return fract( sin( sn ) * c );

}

#ifdef HIGH_PRECISION
	float precisionSafeLength( vec3 v ) { return length( v ); }
#else
	float precisionSafeLength( vec3 v ) {
		float maxComponent = max3( abs( v ) );
		return length( v / maxComponent ) * maxComponent;
	}
#endif

struct IncidentLight {
	vec3 color;
	vec3 direction;
	bool visible;
};

struct ReflectedLight {
	vec3 directDiffuse;
	vec3 directSpecular;
	vec3 indirectDiffuse;
	vec3 indirectSpecular;
};

#ifdef USE_ALPHAHASH

	varying vec3 vPosition;

#endif

vec3 transformDirection( in vec3 dir, in mat4 matrix ) {

	return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );

}

vec3 inverseTransformDirection( in vec3 dir, in mat4 matrix ) {

	// dir can be either a direction vector or a normal vector
	// upper-left 3x3 of matrix is assumed to be orthogonal

	return normalize( ( vec4( dir, 0.0 ) * matrix ).xyz );

}

mat3 transposeMat3( const in mat3 m ) {

	mat3 tmp;

	tmp[ 0 ] = vec3( m[ 0 ].x, m[ 1 ].x, m[ 2 ].x );
	tmp[ 1 ] = vec3( m[ 0 ].y, m[ 1 ].y, m[ 2 ].y );
	tmp[ 2 ] = vec3( m[ 0 ].z, m[ 1 ].z, m[ 2 ].z );

	return tmp;

}

float luminance( const in vec3 rgb ) {

	// assumes rgb is in linear color space with sRGB primaries and D65 white point

	const vec3 weights = vec3( 0.2126729, 0.7151522, 0.0721750 );

	return dot( weights, rgb );

}

bool isPerspectiveMatrix( mat4 m ) {

	return m[ 2 ][ 3 ] == - 1.0;

}

vec2 equirectUv( in vec3 dir ) {

	// dir is assumed to be unit length

	float u = atan( dir.z, dir.x ) * RECIPROCAL_PI2 + 0.5;

	float v = asin( clamp( dir.y, - 1.0, 1.0 ) ) * RECIPROCAL_PI + 0.5;

	return vec2( u, v );

}

vec3 BRDF_Lambert( const in vec3 diffuseColor ) {

	return RECIPROCAL_PI * diffuseColor;

} // validated

vec3 F_Schlick( const in vec3 f0, const in float f90, const in float dotVH ) {

	// Original approximation by Christophe Schlick '94
	// float fresnel = pow( 1.0 - dotVH, 5.0 );

	// Optimized variant (presented by Epic at SIGGRAPH '13)
	// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );

	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );

} // validated

float F_Schlick( const in float f0, const in float f90, const in float dotVH ) {

	// Original approximation by Christophe Schlick '94
	// float fresnel = pow( 1.0 - dotVH, 5.0 );

	// Optimized variant (presented by Epic at SIGGRAPH '13)
	// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );

	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );

} // validated
`;var ku=`
#ifdef ENVMAP_TYPE_CUBE_UV

	#define cubeUV_minMipLevel 4.0
	#define cubeUV_minTileSize 16.0

	// These shader functions convert between the UV coordinates of a single face of
	// a cubemap, the 0-5 integer index of a cube face, and the direction vector for
	// sampling a textureCube (not generally normalized ).

	float getFace( vec3 direction ) {

		vec3 absDirection = abs( direction );

		float face = - 1.0;

		if ( absDirection.x > absDirection.z ) {

			if ( absDirection.x > absDirection.y )

				face = direction.x > 0.0 ? 0.0 : 3.0;

			else

				face = direction.y > 0.0 ? 1.0 : 4.0;

		} else {

			if ( absDirection.z > absDirection.y )

				face = direction.z > 0.0 ? 2.0 : 5.0;

			else

				face = direction.y > 0.0 ? 1.0 : 4.0;

		}

		return face;

	}

	// RH coordinate system; PMREM face-indexing convention
	vec2 getUV( vec3 direction, float face ) {

		vec2 uv;

		if ( face == 0.0 ) {

			uv = vec2( direction.z, direction.y ) / abs( direction.x ); // pos x

		} else if ( face == 1.0 ) {

			uv = vec2( - direction.x, - direction.z ) / abs( direction.y ); // pos y

		} else if ( face == 2.0 ) {

			uv = vec2( - direction.x, direction.y ) / abs( direction.z ); // pos z

		} else if ( face == 3.0 ) {

			uv = vec2( - direction.z, direction.y ) / abs( direction.x ); // neg x

		} else if ( face == 4.0 ) {

			uv = vec2( - direction.x, direction.z ) / abs( direction.y ); // neg y

		} else {

			uv = vec2( direction.x, direction.y ) / abs( direction.z ); // neg z

		}

		return 0.5 * ( uv + 1.0 );

	}

	vec3 bilinearCubeUV( sampler2D envMap, vec3 direction, float mipInt ) {

		float face = getFace( direction );

		float filterInt = max( cubeUV_minMipLevel - mipInt, 0.0 );

		mipInt = max( mipInt, cubeUV_minMipLevel );

		float faceSize = exp2( mipInt );

		highp vec2 uv = getUV( direction, face ) * ( faceSize - 2.0 ) + 1.0; // #25071

		if ( face > 2.0 ) {

			uv.y += faceSize;

			face -= 3.0;

		}

		uv.x += face * faceSize;

		uv.x += filterInt * 3.0 * cubeUV_minTileSize;

		uv.y += 4.0 * ( exp2( CUBEUV_MAX_MIP ) - faceSize );

		uv.x *= CUBEUV_TEXEL_WIDTH;
		uv.y *= CUBEUV_TEXEL_HEIGHT;

		#ifdef texture2DGradEXT

			return texture2DGradEXT( envMap, uv, vec2( 0.0 ), vec2( 0.0 ) ).rgb; // disable anisotropic filtering

		#else

			return texture2D( envMap, uv ).rgb;

		#endif

	}

	// These defines must match with PMREMGenerator

	#define cubeUV_r0 1.0
	#define cubeUV_m0 - 2.0
	#define cubeUV_r1 0.8
	#define cubeUV_m1 - 1.0
	#define cubeUV_r4 0.4
	#define cubeUV_m4 2.0
	#define cubeUV_r5 0.305
	#define cubeUV_m5 3.0
	#define cubeUV_r6 0.21
	#define cubeUV_m6 4.0

	float roughnessToMip( float roughness ) {

		float mip = 0.0;

		if ( roughness >= cubeUV_r1 ) {

			mip = ( cubeUV_r0 - roughness ) * ( cubeUV_m1 - cubeUV_m0 ) / ( cubeUV_r0 - cubeUV_r1 ) + cubeUV_m0;

		} else if ( roughness >= cubeUV_r4 ) {

			mip = ( cubeUV_r1 - roughness ) * ( cubeUV_m4 - cubeUV_m1 ) / ( cubeUV_r1 - cubeUV_r4 ) + cubeUV_m1;

		} else if ( roughness >= cubeUV_r5 ) {

			mip = ( cubeUV_r4 - roughness ) * ( cubeUV_m5 - cubeUV_m4 ) / ( cubeUV_r4 - cubeUV_r5 ) + cubeUV_m4;

		} else if ( roughness >= cubeUV_r6 ) {

			mip = ( cubeUV_r5 - roughness ) * ( cubeUV_m6 - cubeUV_m5 ) / ( cubeUV_r5 - cubeUV_r6 ) + cubeUV_m5;

		} else {

			mip = - 2.0 * log2( 1.16 * roughness ); // 1.16 = 1.79^0.25
		}

		return mip;

	}

	vec4 textureCubeUV( sampler2D envMap, vec3 sampleDir, float roughness ) {

		float mip = clamp( roughnessToMip( roughness ), cubeUV_m0, CUBEUV_MAX_MIP );

		float mipF = fract( mip );

		float mipInt = floor( mip );

		vec3 color0 = bilinearCubeUV( envMap, sampleDir, mipInt );

		if ( mipF == 0.0 ) {

			return vec4( color0, 1.0 );

		} else {

			vec3 color1 = bilinearCubeUV( envMap, sampleDir, mipInt + 1.0 );

			return vec4( mix( color0, color1, mipF ), 1.0 );

		}

	}

#endif
`;var zu=`

vec3 transformedNormal = objectNormal;
#ifdef USE_TANGENT

	vec3 transformedTangent = objectTangent;

#endif

#ifdef USE_BATCHING

	// this is in lieu of a per-instance normal-matrix
	// shear transforms in the instance matrix are not supported

	mat3 bm = mat3( batchingMatrix );
	transformedNormal /= vec3( dot( bm[ 0 ], bm[ 0 ] ), dot( bm[ 1 ], bm[ 1 ] ), dot( bm[ 2 ], bm[ 2 ] ) );
	transformedNormal = bm * transformedNormal;

	#ifdef USE_TANGENT

		transformedTangent = bm * transformedTangent;

	#endif

#endif

#ifdef USE_INSTANCING

	// this is in lieu of a per-instance normal-matrix
	// shear transforms in the instance matrix are not supported

	mat3 im = mat3( instanceMatrix );
	transformedNormal /= vec3( dot( im[ 0 ], im[ 0 ] ), dot( im[ 1 ], im[ 1 ] ), dot( im[ 2 ], im[ 2 ] ) );
	transformedNormal = im * transformedNormal;

	#ifdef USE_TANGENT

		transformedTangent = im * transformedTangent;

	#endif

#endif

transformedNormal = normalMatrix * transformedNormal;

#ifdef FLIP_SIDED

	transformedNormal = - transformedNormal;

#endif

#ifdef USE_TANGENT

	transformedTangent = ( modelViewMatrix * vec4( transformedTangent, 0.0 ) ).xyz;

	#ifdef FLIP_SIDED

		transformedTangent = - transformedTangent;

	#endif

#endif
`;var Vu=`
#ifdef USE_DISPLACEMENTMAP

	uniform sampler2D displacementMap;
	uniform float displacementScale;
	uniform float displacementBias;

#endif
`;var Gu=`
#ifdef USE_DISPLACEMENTMAP

	transformed += normalize( objectNormal ) * ( texture2D( displacementMap, vDisplacementMapUv ).x * displacementScale + displacementBias );

#endif
`;var Wu=`
#ifdef USE_EMISSIVEMAP

	vec4 emissiveColor = texture2D( emissiveMap, vEmissiveMapUv );

	totalEmissiveRadiance *= emissiveColor.rgb;

#endif
`;var Hu=`
#ifdef USE_EMISSIVEMAP

	uniform sampler2D emissiveMap;

#endif
`;var ju=`
gl_FragColor = linearToOutputTexel( gl_FragColor );
`;var Xu=`

// http://www.russellcottrell.com/photo/matrixCalculator.htm

// Linear sRGB => XYZ => Linear Display P3
const mat3 LINEAR_SRGB_TO_LINEAR_DISPLAY_P3 = mat3(
	vec3( 0.8224621, 0.177538, 0.0 ),
	vec3( 0.0331941, 0.9668058, 0.0 ),
	vec3( 0.0170827, 0.0723974, 0.9105199 )
);

// Linear Display P3 => XYZ => Linear sRGB
const mat3 LINEAR_DISPLAY_P3_TO_LINEAR_SRGB = mat3(
	vec3( 1.2249401, - 0.2249404, 0.0 ),
	vec3( - 0.0420569, 1.0420571, 0.0 ),
	vec3( - 0.0196376, - 0.0786361, 1.0982735 )
);

vec4 LinearSRGBToLinearDisplayP3( in vec4 value ) {
	return vec4( value.rgb * LINEAR_SRGB_TO_LINEAR_DISPLAY_P3, value.a );
}

vec4 LinearDisplayP3ToLinearSRGB( in vec4 value ) {
	return vec4( value.rgb * LINEAR_DISPLAY_P3_TO_LINEAR_SRGB, value.a );
}

vec4 LinearTransferOETF( in vec4 value ) {
	return value;
}

vec4 sRGBTransferOETF( in vec4 value ) {
	return vec4( mix( pow( value.rgb, vec3( 0.41666 ) ) * 1.055 - vec3( 0.055 ), value.rgb * 12.92, vec3( lessThanEqual( value.rgb, vec3( 0.0031308 ) ) ) ), value.a );
}

// @deprecated, r156
vec4 LinearToLinear( in vec4 value ) {
	return value;
}

// @deprecated, r156
vec4 LinearTosRGB( in vec4 value ) {
	return sRGBTransferOETF( value );
}
`;var qu=`
#ifdef USE_ENVMAP

	#ifdef ENV_WORLDPOS

		vec3 cameraToFrag;

		if ( isOrthographic ) {

			cameraToFrag = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );

		} else {

			cameraToFrag = normalize( vWorldPosition - cameraPosition );

		}

		// Transforming Normal Vectors with the Inverse Transformation
		vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );

		#ifdef ENVMAP_MODE_REFLECTION

			vec3 reflectVec = reflect( cameraToFrag, worldNormal );

		#else

			vec3 reflectVec = refract( cameraToFrag, worldNormal, refractionRatio );

		#endif

	#else

		vec3 reflectVec = vReflect;

	#endif

	#ifdef ENVMAP_TYPE_CUBE

		vec4 envColor = textureCube( envMap, envMapRotation * vec3( flipEnvMap * reflectVec.x, reflectVec.yz ) );

	#else

		vec4 envColor = vec4( 0.0 );

	#endif

	#ifdef ENVMAP_BLENDING_MULTIPLY

		outgoingLight = mix( outgoingLight, outgoingLight * envColor.xyz, specularStrength * reflectivity );

	#elif defined( ENVMAP_BLENDING_MIX )

		outgoingLight = mix( outgoingLight, envColor.xyz, specularStrength * reflectivity );

	#elif defined( ENVMAP_BLENDING_ADD )

		outgoingLight += envColor.xyz * specularStrength * reflectivity;

	#endif

#endif
`;var $u=`
#ifdef USE_ENVMAP

	uniform float envMapIntensity;
	uniform float flipEnvMap;
	uniform mat3 envMapRotation;

	#ifdef ENVMAP_TYPE_CUBE
		uniform samplerCube envMap;
	#else
		uniform sampler2D envMap;
	#endif
	
#endif
`;var Yu=`
#ifdef USE_ENVMAP

	uniform float reflectivity;

	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )

		#define ENV_WORLDPOS

	#endif

	#ifdef ENV_WORLDPOS

		varying vec3 vWorldPosition;
		uniform float refractionRatio;
	#else
		varying vec3 vReflect;
	#endif

#endif
`;var Ku=`
#ifdef USE_ENVMAP

	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )

		#define ENV_WORLDPOS

	#endif

	#ifdef ENV_WORLDPOS
		
		varying vec3 vWorldPosition;

	#else

		varying vec3 vReflect;
		uniform float refractionRatio;

	#endif

#endif
`;var Zu=`
#ifdef USE_ENVMAP

	#ifdef ENV_WORLDPOS

		vWorldPosition = worldPosition.xyz;

	#else

		vec3 cameraToVertex;

		if ( isOrthographic ) {

			cameraToVertex = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );

		} else {

			cameraToVertex = normalize( worldPosition.xyz - cameraPosition );

		}

		vec3 worldNormal = inverseTransformDirection( transformedNormal, viewMatrix );

		#ifdef ENVMAP_MODE_REFLECTION

			vReflect = reflect( cameraToVertex, worldNormal );

		#else

			vReflect = refract( cameraToVertex, worldNormal, refractionRatio );

		#endif

	#endif

#endif
`;var Ju=`
#ifdef USE_FOG

	vFogDepth = - mvPosition.z;

#endif
`;var Qu=`
#ifdef USE_FOG

	varying float vFogDepth;

#endif
`;var eh=`
#ifdef USE_FOG

	#ifdef FOG_EXP2

		float fogFactor = 1.0 - exp( - fogDensity * fogDensity * vFogDepth * vFogDepth );

	#else

		float fogFactor = smoothstep( fogNear, fogFar, vFogDepth );

	#endif

	gl_FragColor.rgb = mix( gl_FragColor.rgb, fogColor, fogFactor );

#endif
`;var th=`
#ifdef USE_FOG

	uniform vec3 fogColor;
	varying float vFogDepth;

	#ifdef FOG_EXP2

		uniform float fogDensity;

	#else

		uniform float fogNear;
		uniform float fogFar;

	#endif

#endif
`;var ih=`

#ifdef USE_GRADIENTMAP

	uniform sampler2D gradientMap;

#endif

vec3 getGradientIrradiance( vec3 normal, vec3 lightDirection ) {

	// dotNL will be from -1.0 to 1.0
	float dotNL = dot( normal, lightDirection );
	vec2 coord = vec2( dotNL * 0.5 + 0.5, 0.0 );

	#ifdef USE_GRADIENTMAP

		return vec3( texture2D( gradientMap, coord ).r );

	#else

		vec2 fw = fwidth( coord ) * 0.5;
		return mix( vec3( 0.7 ), vec3( 1.0 ), smoothstep( 0.7 - fw.x, 0.7 + fw.x, coord.x ) );

	#endif

}
`;var rh=`
#ifdef USE_LIGHTMAP

	uniform sampler2D lightMap;
	uniform float lightMapIntensity;

#endif
`;var nh=`
LambertMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularStrength = specularStrength;
`;var oh=`
varying vec3 vViewPosition;

struct LambertMaterial {

	vec3 diffuseColor;
	float specularStrength;

};

void RE_Direct_Lambert( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {

	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;

	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

void RE_IndirectDiffuse_Lambert( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {

	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

#define RE_Direct				RE_Direct_Lambert
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Lambert
`;var sh=`
uniform bool receiveShadow;
uniform vec3 ambientLightColor;

#if defined( USE_LIGHT_PROBES )

	uniform vec3 lightProbe[ 9 ];

#endif

// get the irradiance (radiance convolved with cosine lobe) at the point 'normal' on the unit sphere
// source: https://graphics.stanford.edu/papers/envmap/envmap.pdf
vec3 shGetIrradianceAt( in vec3 normal, in vec3 shCoefficients[ 9 ] ) {

	// normal is assumed to have unit length

	float x = normal.x, y = normal.y, z = normal.z;

	// band 0
	vec3 result = shCoefficients[ 0 ] * 0.886227;

	// band 1
	result += shCoefficients[ 1 ] * 2.0 * 0.511664 * y;
	result += shCoefficients[ 2 ] * 2.0 * 0.511664 * z;
	result += shCoefficients[ 3 ] * 2.0 * 0.511664 * x;

	// band 2
	result += shCoefficients[ 4 ] * 2.0 * 0.429043 * x * y;
	result += shCoefficients[ 5 ] * 2.0 * 0.429043 * y * z;
	result += shCoefficients[ 6 ] * ( 0.743125 * z * z - 0.247708 );
	result += shCoefficients[ 7 ] * 2.0 * 0.429043 * x * z;
	result += shCoefficients[ 8 ] * 0.429043 * ( x * x - y * y );

	return result;

}

vec3 getLightProbeIrradiance( const in vec3 lightProbe[ 9 ], const in vec3 normal ) {

	vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );

	vec3 irradiance = shGetIrradianceAt( worldNormal, lightProbe );

	return irradiance;

}

vec3 getAmbientLightIrradiance( const in vec3 ambientLightColor ) {

	vec3 irradiance = ambientLightColor;

	return irradiance;

}

float getDistanceAttenuation( const in float lightDistance, const in float cutoffDistance, const in float decayExponent ) {

	#if defined ( LEGACY_LIGHTS )

		if ( cutoffDistance > 0.0 && decayExponent > 0.0 ) {

			return pow( saturate( - lightDistance / cutoffDistance + 1.0 ), decayExponent );

		}

		return 1.0;

	#else

		// based upon Frostbite 3 Moving to Physically-based Rendering
		// page 32, equation 26: E[window1]
		// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
		float distanceFalloff = 1.0 / max( pow( lightDistance, decayExponent ), 0.01 );

		if ( cutoffDistance > 0.0 ) {

			distanceFalloff *= pow2( saturate( 1.0 - pow4( lightDistance / cutoffDistance ) ) );

		}

		return distanceFalloff;

	#endif

}

float getSpotAttenuation( const in float coneCosine, const in float penumbraCosine, const in float angleCosine ) {

	return smoothstep( coneCosine, penumbraCosine, angleCosine );

}

#if NUM_DIR_LIGHTS > 0

	struct DirectionalLight {
		vec3 direction;
		vec3 color;
	};

	uniform DirectionalLight directionalLights[ NUM_DIR_LIGHTS ];

	void getDirectionalLightInfo( const in DirectionalLight directionalLight, out IncidentLight light ) {

		light.color = directionalLight.color;
		light.direction = directionalLight.direction;
		light.visible = true;

	}

#endif


#if NUM_POINT_LIGHTS > 0

	struct PointLight {
		vec3 position;
		vec3 color;
		float distance;
		float decay;
	};

	uniform PointLight pointLights[ NUM_POINT_LIGHTS ];

	// light is an out parameter as having it as a return value caused compiler errors on some devices
	void getPointLightInfo( const in PointLight pointLight, const in vec3 geometryPosition, out IncidentLight light ) {

		vec3 lVector = pointLight.position - geometryPosition;

		light.direction = normalize( lVector );

		float lightDistance = length( lVector );

		light.color = pointLight.color;
		light.color *= getDistanceAttenuation( lightDistance, pointLight.distance, pointLight.decay );
		light.visible = ( light.color != vec3( 0.0 ) );

	}

#endif


#if NUM_SPOT_LIGHTS > 0

	struct SpotLight {
		vec3 position;
		vec3 direction;
		vec3 color;
		float distance;
		float decay;
		float coneCos;
		float penumbraCos;
	};

	uniform SpotLight spotLights[ NUM_SPOT_LIGHTS ];

	// light is an out parameter as having it as a return value caused compiler errors on some devices
	void getSpotLightInfo( const in SpotLight spotLight, const in vec3 geometryPosition, out IncidentLight light ) {

		vec3 lVector = spotLight.position - geometryPosition;

		light.direction = normalize( lVector );

		float angleCos = dot( light.direction, spotLight.direction );

		float spotAttenuation = getSpotAttenuation( spotLight.coneCos, spotLight.penumbraCos, angleCos );

		if ( spotAttenuation > 0.0 ) {

			float lightDistance = length( lVector );

			light.color = spotLight.color * spotAttenuation;
			light.color *= getDistanceAttenuation( lightDistance, spotLight.distance, spotLight.decay );
			light.visible = ( light.color != vec3( 0.0 ) );

		} else {

			light.color = vec3( 0.0 );
			light.visible = false;

		}

	}

#endif


#if NUM_RECT_AREA_LIGHTS > 0

	struct RectAreaLight {
		vec3 color;
		vec3 position;
		vec3 halfWidth;
		vec3 halfHeight;
	};

	// Pre-computed values of LinearTransformedCosine approximation of BRDF
	// BRDF approximation Texture is 64x64
	uniform sampler2D ltc_1; // RGBA Float
	uniform sampler2D ltc_2; // RGBA Float

	uniform RectAreaLight rectAreaLights[ NUM_RECT_AREA_LIGHTS ];

#endif


#if NUM_HEMI_LIGHTS > 0

	struct HemisphereLight {
		vec3 direction;
		vec3 skyColor;
		vec3 groundColor;
	};

	uniform HemisphereLight hemisphereLights[ NUM_HEMI_LIGHTS ];

	vec3 getHemisphereLightIrradiance( const in HemisphereLight hemiLight, const in vec3 normal ) {

		float dotNL = dot( normal, hemiLight.direction );
		float hemiDiffuseWeight = 0.5 * dotNL + 0.5;

		vec3 irradiance = mix( hemiLight.groundColor, hemiLight.skyColor, hemiDiffuseWeight );

		return irradiance;

	}

#endif
`;var ah=`
#ifdef USE_ENVMAP

	vec3 getIBLIrradiance( const in vec3 normal ) {

		#ifdef ENVMAP_TYPE_CUBE_UV

			vec3 worldNormal = inverseTransformDirection( normal, viewMatrix );

			vec4 envMapColor = textureCubeUV( envMap, envMapRotation * worldNormal, 1.0 );

			return PI * envMapColor.rgb * envMapIntensity;

		#else

			return vec3( 0.0 );

		#endif

	}

	vec3 getIBLRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness ) {

		#ifdef ENVMAP_TYPE_CUBE_UV

			vec3 reflectVec = reflect( - viewDir, normal );

			// Mixing the reflection with the normal is more accurate and keeps rough objects from gathering light from behind their tangent plane.
			reflectVec = normalize( mix( reflectVec, normal, roughness * roughness) );

			reflectVec = inverseTransformDirection( reflectVec, viewMatrix );

			vec4 envMapColor = textureCubeUV( envMap, envMapRotation * reflectVec, roughness );

			return envMapColor.rgb * envMapIntensity;

		#else

			return vec3( 0.0 );

		#endif

	}

	#ifdef USE_ANISOTROPY

		vec3 getIBLAnisotropyRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness, const in vec3 bitangent, const in float anisotropy ) {

			#ifdef ENVMAP_TYPE_CUBE_UV

			  // https://google.github.io/filament/Filament.md.html#lighting/imagebasedlights/anisotropy
				vec3 bentNormal = cross( bitangent, viewDir );
				bentNormal = normalize( cross( bentNormal, bitangent ) );
				bentNormal = normalize( mix( bentNormal, normal, pow2( pow2( 1.0 - anisotropy * ( 1.0 - roughness ) ) ) ) );

				return getIBLRadiance( viewDir, bentNormal, roughness );

			#else

				return vec3( 0.0 );

			#endif

		}

	#endif

#endif
`;var ch=`
ToonMaterial material;
material.diffuseColor = diffuseColor.rgb;
`;var lh=`
varying vec3 vViewPosition;

struct ToonMaterial {

	vec3 diffuseColor;

};

void RE_Direct_Toon( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {

	vec3 irradiance = getGradientIrradiance( geometryNormal, directLight.direction ) * directLight.color;

	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

void RE_IndirectDiffuse_Toon( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {

	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

#define RE_Direct				RE_Direct_Toon
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Toon
`;var dh=`
BlinnPhongMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularColor = specular;
material.specularShininess = shininess;
material.specularStrength = specularStrength;
`;var uh=`
varying vec3 vViewPosition;

struct BlinnPhongMaterial {

	vec3 diffuseColor;
	vec3 specularColor;
	float specularShininess;
	float specularStrength;

};

void RE_Direct_BlinnPhong( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {

	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;

	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

	reflectedLight.directSpecular += irradiance * BRDF_BlinnPhong( directLight.direction, geometryViewDir, geometryNormal, material.specularColor, material.specularShininess ) * material.specularStrength;

}

void RE_IndirectDiffuse_BlinnPhong( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {

	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

#define RE_Direct				RE_Direct_BlinnPhong
#define RE_IndirectDiffuse		RE_IndirectDiffuse_BlinnPhong
`;var hh=`
PhysicalMaterial material;
material.diffuseColor = diffuseColor.rgb * ( 1.0 - metalnessFactor );

vec3 dxy = max( abs( dFdx( nonPerturbedNormal ) ), abs( dFdy( nonPerturbedNormal ) ) );
float geometryRoughness = max( max( dxy.x, dxy.y ), dxy.z );

material.roughness = max( roughnessFactor, 0.0525 );// 0.0525 corresponds to the base mip of a 256 cubemap.
material.roughness += geometryRoughness;
material.roughness = min( material.roughness, 1.0 );

#ifdef IOR

	material.ior = ior;

	#ifdef USE_SPECULAR

		float specularIntensityFactor = specularIntensity;
		vec3 specularColorFactor = specularColor;

		#ifdef USE_SPECULAR_COLORMAP

			specularColorFactor *= texture2D( specularColorMap, vSpecularColorMapUv ).rgb;

		#endif

		#ifdef USE_SPECULAR_INTENSITYMAP

			specularIntensityFactor *= texture2D( specularIntensityMap, vSpecularIntensityMapUv ).a;

		#endif

		material.specularF90 = mix( specularIntensityFactor, 1.0, metalnessFactor );

	#else

		float specularIntensityFactor = 1.0;
		vec3 specularColorFactor = vec3( 1.0 );
		material.specularF90 = 1.0;

	#endif

	material.specularColor = mix( min( pow2( ( material.ior - 1.0 ) / ( material.ior + 1.0 ) ) * specularColorFactor, vec3( 1.0 ) ) * specularIntensityFactor, diffuseColor.rgb, metalnessFactor );

#else

	material.specularColor = mix( vec3( 0.04 ), diffuseColor.rgb, metalnessFactor );
	material.specularF90 = 1.0;

#endif

#ifdef USE_CLEARCOAT

	material.clearcoat = clearcoat;
	material.clearcoatRoughness = clearcoatRoughness;
	material.clearcoatF0 = vec3( 0.04 );
	material.clearcoatF90 = 1.0;

	#ifdef USE_CLEARCOATMAP

		material.clearcoat *= texture2D( clearcoatMap, vClearcoatMapUv ).x;

	#endif

	#ifdef USE_CLEARCOAT_ROUGHNESSMAP

		material.clearcoatRoughness *= texture2D( clearcoatRoughnessMap, vClearcoatRoughnessMapUv ).y;

	#endif

	material.clearcoat = saturate( material.clearcoat ); // Burley clearcoat model
	material.clearcoatRoughness = max( material.clearcoatRoughness, 0.0525 );
	material.clearcoatRoughness += geometryRoughness;
	material.clearcoatRoughness = min( material.clearcoatRoughness, 1.0 );

#endif

#ifdef USE_DISPERSION

	material.dispersion = dispersion;

#endif

#ifdef USE_IRIDESCENCE

	material.iridescence = iridescence;
	material.iridescenceIOR = iridescenceIOR;

	#ifdef USE_IRIDESCENCEMAP

		material.iridescence *= texture2D( iridescenceMap, vIridescenceMapUv ).r;

	#endif

	#ifdef USE_IRIDESCENCE_THICKNESSMAP

		material.iridescenceThickness = (iridescenceThicknessMaximum - iridescenceThicknessMinimum) * texture2D( iridescenceThicknessMap, vIridescenceThicknessMapUv ).g + iridescenceThicknessMinimum;

	#else

		material.iridescenceThickness = iridescenceThicknessMaximum;

	#endif

#endif

#ifdef USE_SHEEN

	material.sheenColor = sheenColor;

	#ifdef USE_SHEEN_COLORMAP

		material.sheenColor *= texture2D( sheenColorMap, vSheenColorMapUv ).rgb;

	#endif

	material.sheenRoughness = clamp( sheenRoughness, 0.07, 1.0 );

	#ifdef USE_SHEEN_ROUGHNESSMAP

		material.sheenRoughness *= texture2D( sheenRoughnessMap, vSheenRoughnessMapUv ).a;

	#endif

#endif

#ifdef USE_ANISOTROPY

	#ifdef USE_ANISOTROPYMAP

		mat2 anisotropyMat = mat2( anisotropyVector.x, anisotropyVector.y, - anisotropyVector.y, anisotropyVector.x );
		vec3 anisotropyPolar = texture2D( anisotropyMap, vAnisotropyMapUv ).rgb;
		vec2 anisotropyV = anisotropyMat * normalize( 2.0 * anisotropyPolar.rg - vec2( 1.0 ) ) * anisotropyPolar.b;

	#else

		vec2 anisotropyV = anisotropyVector;

	#endif

	material.anisotropy = length( anisotropyV );

	if( material.anisotropy == 0.0 ) {
		anisotropyV = vec2( 1.0, 0.0 );
	} else {
		anisotropyV /= material.anisotropy;
		material.anisotropy = saturate( material.anisotropy );
	}

	// Roughness along the anisotropy bitangent is the material roughness, while the tangent roughness increases with anisotropy.
	material.alphaT = mix( pow2( material.roughness ), 1.0, pow2( material.anisotropy ) );

	material.anisotropyT = tbn[ 0 ] * anisotropyV.x + tbn[ 1 ] * anisotropyV.y;
	material.anisotropyB = tbn[ 1 ] * anisotropyV.x - tbn[ 0 ] * anisotropyV.y;

#endif
`;var fh=`

struct PhysicalMaterial {

	vec3 diffuseColor;
	float roughness;
	vec3 specularColor;
	float specularF90;
	float dispersion;

	#ifdef USE_CLEARCOAT
		float clearcoat;
		float clearcoatRoughness;
		vec3 clearcoatF0;
		float clearcoatF90;
	#endif

	#ifdef USE_IRIDESCENCE
		float iridescence;
		float iridescenceIOR;
		float iridescenceThickness;
		vec3 iridescenceFresnel;
		vec3 iridescenceF0;
	#endif

	#ifdef USE_SHEEN
		vec3 sheenColor;
		float sheenRoughness;
	#endif

	#ifdef IOR
		float ior;
	#endif

	#ifdef USE_TRANSMISSION
		float transmission;
		float transmissionAlpha;
		float thickness;
		float attenuationDistance;
		vec3 attenuationColor;
	#endif

	#ifdef USE_ANISOTROPY
		float anisotropy;
		float alphaT;
		vec3 anisotropyT;
		vec3 anisotropyB;
	#endif

};

// temporary
vec3 clearcoatSpecularDirect = vec3( 0.0 );
vec3 clearcoatSpecularIndirect = vec3( 0.0 );
vec3 sheenSpecularDirect = vec3( 0.0 );
vec3 sheenSpecularIndirect = vec3(0.0 );

vec3 Schlick_to_F0( const in vec3 f, const in float f90, const in float dotVH ) {
    float x = clamp( 1.0 - dotVH, 0.0, 1.0 );
    float x2 = x * x;
    float x5 = clamp( x * x2 * x2, 0.0, 0.9999 );

    return ( f - vec3( f90 ) * x5 ) / ( 1.0 - x5 );
}

// Moving Frostbite to Physically Based Rendering 3.0 - page 12, listing 2
// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
float V_GGX_SmithCorrelated( const in float alpha, const in float dotNL, const in float dotNV ) {

	float a2 = pow2( alpha );

	float gv = dotNL * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNV ) );
	float gl = dotNV * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNL ) );

	return 0.5 / max( gv + gl, EPSILON );

}

// Microfacet Models for Refraction through Rough Surfaces - equation (33)
// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// alpha is "roughness squared" in Disney\u2019s reparameterization
float D_GGX( const in float alpha, const in float dotNH ) {

	float a2 = pow2( alpha );

	float denom = pow2( dotNH ) * ( a2 - 1.0 ) + 1.0; // avoid alpha = 0 with dotNH = 1

	return RECIPROCAL_PI * a2 / pow2( denom );

}

// https://google.github.io/filament/Filament.md.html#materialsystem/anisotropicmodel/anisotropicspecularbrdf
#ifdef USE_ANISOTROPY

	float V_GGX_SmithCorrelated_Anisotropic( const in float alphaT, const in float alphaB, const in float dotTV, const in float dotBV, const in float dotTL, const in float dotBL, const in float dotNV, const in float dotNL ) {

		float gv = dotNL * length( vec3( alphaT * dotTV, alphaB * dotBV, dotNV ) );
		float gl = dotNV * length( vec3( alphaT * dotTL, alphaB * dotBL, dotNL ) );
		float v = 0.5 / ( gv + gl );

		return saturate(v);

	}

	float D_GGX_Anisotropic( const in float alphaT, const in float alphaB, const in float dotNH, const in float dotTH, const in float dotBH ) {

		float a2 = alphaT * alphaB;
		highp vec3 v = vec3( alphaB * dotTH, alphaT * dotBH, a2 * dotNH );
		highp float v2 = dot( v, v );
		float w2 = a2 / v2;

		return RECIPROCAL_PI * a2 * pow2 ( w2 );

	}

#endif

#ifdef USE_CLEARCOAT

	// GGX Distribution, Schlick Fresnel, GGX_SmithCorrelated Visibility
	vec3 BRDF_GGX_Clearcoat( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material) {

		vec3 f0 = material.clearcoatF0;
		float f90 = material.clearcoatF90;
		float roughness = material.clearcoatRoughness;

		float alpha = pow2( roughness ); // UE4's roughness

		vec3 halfDir = normalize( lightDir + viewDir );

		float dotNL = saturate( dot( normal, lightDir ) );
		float dotNV = saturate( dot( normal, viewDir ) );
		float dotNH = saturate( dot( normal, halfDir ) );
		float dotVH = saturate( dot( viewDir, halfDir ) );

		vec3 F = F_Schlick( f0, f90, dotVH );

		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );

		float D = D_GGX( alpha, dotNH );

		return F * ( V * D );

	}

#endif

vec3 BRDF_GGX( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material ) {

	vec3 f0 = material.specularColor;
	float f90 = material.specularF90;
	float roughness = material.roughness;

	float alpha = pow2( roughness ); // UE4's roughness

	vec3 halfDir = normalize( lightDir + viewDir );

	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );

	vec3 F = F_Schlick( f0, f90, dotVH );

	#ifdef USE_IRIDESCENCE

		F = mix( F, material.iridescenceFresnel, material.iridescence );

	#endif

	#ifdef USE_ANISOTROPY

		float dotTL = dot( material.anisotropyT, lightDir );
		float dotTV = dot( material.anisotropyT, viewDir );
		float dotTH = dot( material.anisotropyT, halfDir );
		float dotBL = dot( material.anisotropyB, lightDir );
		float dotBV = dot( material.anisotropyB, viewDir );
		float dotBH = dot( material.anisotropyB, halfDir );

		float V = V_GGX_SmithCorrelated_Anisotropic( material.alphaT, alpha, dotTV, dotBV, dotTL, dotBL, dotNV, dotNL );

		float D = D_GGX_Anisotropic( material.alphaT, alpha, dotNH, dotTH, dotBH );

	#else

		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );

		float D = D_GGX( alpha, dotNH );

	#endif

	return F * ( V * D );

}

// Rect Area Light

// Real-Time Polygonal-Light Shading with Linearly Transformed Cosines
// by Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt
// code: https://github.com/selfshadow/ltc_code/

vec2 LTC_Uv( const in vec3 N, const in vec3 V, const in float roughness ) {

	const float LUT_SIZE = 64.0;
	const float LUT_SCALE = ( LUT_SIZE - 1.0 ) / LUT_SIZE;
	const float LUT_BIAS = 0.5 / LUT_SIZE;

	float dotNV = saturate( dot( N, V ) );

	// texture parameterized by sqrt( GGX alpha ) and sqrt( 1 - cos( theta ) )
	vec2 uv = vec2( roughness, sqrt( 1.0 - dotNV ) );

	uv = uv * LUT_SCALE + LUT_BIAS;

	return uv;

}

float LTC_ClippedSphereFormFactor( const in vec3 f ) {

	// Real-Time Area Lighting: a Journey from Research to Production (p.102)
	// An approximation of the form factor of a horizon-clipped rectangle.

	float l = length( f );

	return max( ( l * l + f.z ) / ( l + 1.0 ), 0.0 );

}

vec3 LTC_EdgeVectorFormFactor( const in vec3 v1, const in vec3 v2 ) {

	float x = dot( v1, v2 );

	float y = abs( x );

	// rational polynomial approximation to theta / sin( theta ) / 2PI
	float a = 0.8543985 + ( 0.4965155 + 0.0145206 * y ) * y;
	float b = 3.4175940 + ( 4.1616724 + y ) * y;
	float v = a / b;

	float theta_sintheta = ( x > 0.0 ) ? v : 0.5 * inversesqrt( max( 1.0 - x * x, 1e-7 ) ) - v;

	return cross( v1, v2 ) * theta_sintheta;

}

vec3 LTC_Evaluate( const in vec3 N, const in vec3 V, const in vec3 P, const in mat3 mInv, const in vec3 rectCoords[ 4 ] ) {

	// bail if point is on back side of plane of light
	// assumes ccw winding order of light vertices
	vec3 v1 = rectCoords[ 1 ] - rectCoords[ 0 ];
	vec3 v2 = rectCoords[ 3 ] - rectCoords[ 0 ];
	vec3 lightNormal = cross( v1, v2 );

	if( dot( lightNormal, P - rectCoords[ 0 ] ) < 0.0 ) return vec3( 0.0 );

	// construct orthonormal basis around N
	vec3 T1, T2;
	T1 = normalize( V - N * dot( V, N ) );
	T2 = - cross( N, T1 ); // negated from paper; possibly due to a different handedness of world coordinate system

	// compute transform
	mat3 mat = mInv * transposeMat3( mat3( T1, T2, N ) );

	// transform rect
	vec3 coords[ 4 ];
	coords[ 0 ] = mat * ( rectCoords[ 0 ] - P );
	coords[ 1 ] = mat * ( rectCoords[ 1 ] - P );
	coords[ 2 ] = mat * ( rectCoords[ 2 ] - P );
	coords[ 3 ] = mat * ( rectCoords[ 3 ] - P );

	// project rect onto sphere
	coords[ 0 ] = normalize( coords[ 0 ] );
	coords[ 1 ] = normalize( coords[ 1 ] );
	coords[ 2 ] = normalize( coords[ 2 ] );
	coords[ 3 ] = normalize( coords[ 3 ] );

	// calculate vector form factor
	vec3 vectorFormFactor = vec3( 0.0 );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 0 ], coords[ 1 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 1 ], coords[ 2 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 2 ], coords[ 3 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 3 ], coords[ 0 ] );

	// adjust for horizon clipping
	float result = LTC_ClippedSphereFormFactor( vectorFormFactor );

/*
	// alternate method of adjusting for horizon clipping (see referece)
	// refactoring required
	float len = length( vectorFormFactor );
	float z = vectorFormFactor.z / len;

	const float LUT_SIZE = 64.0;
	const float LUT_SCALE = ( LUT_SIZE - 1.0 ) / LUT_SIZE;
	const float LUT_BIAS = 0.5 / LUT_SIZE;

	// tabulated horizon-clipped sphere, apparently...
	vec2 uv = vec2( z * 0.5 + 0.5, len );
	uv = uv * LUT_SCALE + LUT_BIAS;

	float scale = texture2D( ltc_2, uv ).w;

	float result = len * scale;
*/

	return vec3( result );

}

// End Rect Area Light

#if defined( USE_SHEEN )

// https://github.com/google/filament/blob/master/shaders/src/brdf.fs
float D_Charlie( float roughness, float dotNH ) {

	float alpha = pow2( roughness );

	// Estevez and Kulla 2017, "Production Friendly Microfacet Sheen BRDF"
	float invAlpha = 1.0 / alpha;
	float cos2h = dotNH * dotNH;
	float sin2h = max( 1.0 - cos2h, 0.0078125 ); // 2^(-14/2), so sin2h^2 > 0 in fp16

	return ( 2.0 + invAlpha ) * pow( sin2h, invAlpha * 0.5 ) / ( 2.0 * PI );

}

// https://github.com/google/filament/blob/master/shaders/src/brdf.fs
float V_Neubelt( float dotNV, float dotNL ) {

	// Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
	return saturate( 1.0 / ( 4.0 * ( dotNL + dotNV - dotNL * dotNV ) ) );

}

vec3 BRDF_Sheen( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, vec3 sheenColor, const in float sheenRoughness ) {

	vec3 halfDir = normalize( lightDir + viewDir );

	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );

	float D = D_Charlie( sheenRoughness, dotNH );
	float V = V_Neubelt( dotNV, dotNL );

	return sheenColor * ( D * V );

}

#endif

// This is a curve-fit approxmation to the "Charlie sheen" BRDF integrated over the hemisphere from 
// Estevez and Kulla 2017, "Production Friendly Microfacet Sheen BRDF". The analysis can be found
// in the Sheen section of https://drive.google.com/file/d/1T0D1VSyR4AllqIJTQAraEIzjlb5h4FKH/view?usp=sharing
float IBLSheenBRDF( const in vec3 normal, const in vec3 viewDir, const in float roughness ) {

	float dotNV = saturate( dot( normal, viewDir ) );

	float r2 = roughness * roughness;

	float a = roughness < 0.25 ? -339.2 * r2 + 161.4 * roughness - 25.9 : -8.48 * r2 + 14.3 * roughness - 9.95;

	float b = roughness < 0.25 ? 44.0 * r2 - 23.7 * roughness + 3.26 : 1.97 * r2 - 3.27 * roughness + 0.72;

	float DG = exp( a * dotNV + b ) + ( roughness < 0.25 ? 0.0 : 0.1 * ( roughness - 0.25 ) );

	return saturate( DG * RECIPROCAL_PI );

}

// Analytical approximation of the DFG LUT, one half of the
// split-sum approximation used in indirect specular lighting.
// via 'environmentBRDF' from "Physically Based Shading on Mobile"
// https://www.unrealengine.com/blog/physically-based-shading-on-mobile
vec2 DFGApprox( const in vec3 normal, const in vec3 viewDir, const in float roughness ) {

	float dotNV = saturate( dot( normal, viewDir ) );

	const vec4 c0 = vec4( - 1, - 0.0275, - 0.572, 0.022 );

	const vec4 c1 = vec4( 1, 0.0425, 1.04, - 0.04 );

	vec4 r = roughness * c0 + c1;

	float a004 = min( r.x * r.x, exp2( - 9.28 * dotNV ) ) * r.x + r.y;

	vec2 fab = vec2( - 1.04, 1.04 ) * a004 + r.zw;

	return fab;

}

vec3 EnvironmentBRDF( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float roughness ) {

	vec2 fab = DFGApprox( normal, viewDir, roughness );

	return specularColor * fab.x + specularF90 * fab.y;

}

// Fdez-Ag\xFCera's "Multiple-Scattering Microfacet Model for Real-Time Image Based Lighting"
// Approximates multiscattering in order to preserve energy.
// http://www.jcgt.org/published/0008/01/03/
#ifdef USE_IRIDESCENCE
void computeMultiscatteringIridescence( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float iridescence, const in vec3 iridescenceF0, const in float roughness, inout vec3 singleScatter, inout vec3 multiScatter ) {
#else
void computeMultiscattering( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float roughness, inout vec3 singleScatter, inout vec3 multiScatter ) {
#endif

	vec2 fab = DFGApprox( normal, viewDir, roughness );

	#ifdef USE_IRIDESCENCE

		vec3 Fr = mix( specularColor, iridescenceF0, iridescence );

	#else

		vec3 Fr = specularColor;

	#endif

	vec3 FssEss = Fr * fab.x + specularF90 * fab.y;

	float Ess = fab.x + fab.y;
	float Ems = 1.0 - Ess;

	vec3 Favg = Fr + ( 1.0 - Fr ) * 0.047619; // 1/21
	vec3 Fms = FssEss * Favg / ( 1.0 - Ems * Favg );

	singleScatter += FssEss;
	multiScatter += Fms * Ems;

}

#if NUM_RECT_AREA_LIGHTS > 0

	void RE_Direct_RectArea_Physical( const in RectAreaLight rectAreaLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {

		vec3 normal = geometryNormal;
		vec3 viewDir = geometryViewDir;
		vec3 position = geometryPosition;
		vec3 lightPos = rectAreaLight.position;
		vec3 halfWidth = rectAreaLight.halfWidth;
		vec3 halfHeight = rectAreaLight.halfHeight;
		vec3 lightColor = rectAreaLight.color;
		float roughness = material.roughness;

		vec3 rectCoords[ 4 ];
		rectCoords[ 0 ] = lightPos + halfWidth - halfHeight; // counterclockwise; light shines in local neg z direction
		rectCoords[ 1 ] = lightPos - halfWidth - halfHeight;
		rectCoords[ 2 ] = lightPos - halfWidth + halfHeight;
		rectCoords[ 3 ] = lightPos + halfWidth + halfHeight;

		vec2 uv = LTC_Uv( normal, viewDir, roughness );

		vec4 t1 = texture2D( ltc_1, uv );
		vec4 t2 = texture2D( ltc_2, uv );

		mat3 mInv = mat3(
			vec3( t1.x, 0, t1.y ),
			vec3(    0, 1,    0 ),
			vec3( t1.z, 0, t1.w )
		);

		// LTC Fresnel Approximation by Stephen Hill
		// http://blog.selfshadow.com/publications/s2016-advances/s2016_ltc_fresnel.pdf
		vec3 fresnel = ( material.specularColor * t2.x + ( vec3( 1.0 ) - material.specularColor ) * t2.y );

		reflectedLight.directSpecular += lightColor * fresnel * LTC_Evaluate( normal, viewDir, position, mInv, rectCoords );

		reflectedLight.directDiffuse += lightColor * material.diffuseColor * LTC_Evaluate( normal, viewDir, position, mat3( 1.0 ), rectCoords );

	}

#endif

void RE_Direct_Physical( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {

	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );

	vec3 irradiance = dotNL * directLight.color;

	#ifdef USE_CLEARCOAT

		float dotNLcc = saturate( dot( geometryClearcoatNormal, directLight.direction ) );

		vec3 ccIrradiance = dotNLcc * directLight.color;

		clearcoatSpecularDirect += ccIrradiance * BRDF_GGX_Clearcoat( directLight.direction, geometryViewDir, geometryClearcoatNormal, material );

	#endif

	#ifdef USE_SHEEN

		sheenSpecularDirect += irradiance * BRDF_Sheen( directLight.direction, geometryViewDir, geometryNormal, material.sheenColor, material.sheenRoughness );

	#endif

	reflectedLight.directSpecular += irradiance * BRDF_GGX( directLight.direction, geometryViewDir, geometryNormal, material );

	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}

void RE_IndirectDiffuse_Physical( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {

	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );

}

void RE_IndirectSpecular_Physical( const in vec3 radiance, const in vec3 irradiance, const in vec3 clearcoatRadiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {

	#ifdef USE_CLEARCOAT

		clearcoatSpecularIndirect += clearcoatRadiance * EnvironmentBRDF( geometryClearcoatNormal, geometryViewDir, material.clearcoatF0, material.clearcoatF90, material.clearcoatRoughness );

	#endif

	#ifdef USE_SHEEN

		sheenSpecularIndirect += irradiance * material.sheenColor * IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );

	#endif

	// Both indirect specular and indirect diffuse light accumulate here

	vec3 singleScattering = vec3( 0.0 );
	vec3 multiScattering = vec3( 0.0 );
	vec3 cosineWeightedIrradiance = irradiance * RECIPROCAL_PI;

	#ifdef USE_IRIDESCENCE

		computeMultiscatteringIridescence( geometryNormal, geometryViewDir, material.specularColor, material.specularF90, material.iridescence, material.iridescenceFresnel, material.roughness, singleScattering, multiScattering );

	#else

		computeMultiscattering( geometryNormal, geometryViewDir, material.specularColor, material.specularF90, material.roughness, singleScattering, multiScattering );

	#endif

	vec3 totalScattering = singleScattering + multiScattering;
	vec3 diffuse = material.diffuseColor * ( 1.0 - max( max( totalScattering.r, totalScattering.g ), totalScattering.b ) );

	reflectedLight.indirectSpecular += radiance * singleScattering;
	reflectedLight.indirectSpecular += multiScattering * cosineWeightedIrradiance;

	reflectedLight.indirectDiffuse += diffuse * cosineWeightedIrradiance;

}

#define RE_Direct				RE_Direct_Physical
#define RE_Direct_RectArea		RE_Direct_RectArea_Physical
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Physical
#define RE_IndirectSpecular		RE_IndirectSpecular_Physical

// ref: https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
float computeSpecularOcclusion( const in float dotNV, const in float ambientOcclusion, const in float roughness ) {

	return saturate( pow( dotNV + ambientOcclusion, exp2( - 16.0 * roughness - 1.0 ) ) - 1.0 + ambientOcclusion );

}
`;var ph=`
/**
 * This is a template that can be used to light a material, it uses pluggable
 * RenderEquations (RE)for specific lighting scenarios.
 *
 * Instructions for use:
 * - Ensure that both RE_Direct, RE_IndirectDiffuse and RE_IndirectSpecular are defined
 * - Create a material parameter that is to be passed as the third parameter to your lighting functions.
 *
 * TODO:
 * - Add area light support.
 * - Add sphere light support.
 * - Add diffuse light probe (irradiance cubemap) support.
 */

vec3 geometryPosition = - vViewPosition;
vec3 geometryNormal = normal;
vec3 geometryViewDir = ( isOrthographic ) ? vec3( 0, 0, 1 ) : normalize( vViewPosition );

vec3 geometryClearcoatNormal = vec3( 0.0 );

#ifdef USE_CLEARCOAT

	geometryClearcoatNormal = clearcoatNormal;

#endif

#ifdef USE_IRIDESCENCE

	float dotNVi = saturate( dot( normal, geometryViewDir ) );

	if ( material.iridescenceThickness == 0.0 ) {

		material.iridescence = 0.0;

	} else {

		material.iridescence = saturate( material.iridescence );

	}

	if ( material.iridescence > 0.0 ) {

		material.iridescenceFresnel = evalIridescence( 1.0, material.iridescenceIOR, dotNVi, material.iridescenceThickness, material.specularColor );

		// Iridescence F0 approximation
		material.iridescenceF0 = Schlick_to_F0( material.iridescenceFresnel, 1.0, dotNVi );

	}

#endif

IncidentLight directLight;

#if ( NUM_POINT_LIGHTS > 0 ) && defined( RE_Direct )

	PointLight pointLight;
	#if defined( USE_SHADOWMAP ) && NUM_POINT_LIGHT_SHADOWS > 0
	PointLightShadow pointLightShadow;
	#endif

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHTS; i ++ ) {

		pointLight = pointLights[ i ];

		getPointLightInfo( pointLight, geometryPosition, directLight );

		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_POINT_LIGHT_SHADOWS )
		pointLightShadow = pointLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getPointShadow( pointShadowMap[ i ], pointLightShadow.shadowMapSize, pointLightShadow.shadowBias, pointLightShadow.shadowRadius, vPointShadowCoord[ i ], pointLightShadow.shadowCameraNear, pointLightShadow.shadowCameraFar ) : 1.0;
		#endif

		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

	}
	#pragma unroll_loop_end

#endif

#if ( NUM_SPOT_LIGHTS > 0 ) && defined( RE_Direct )

	SpotLight spotLight;
	vec4 spotColor;
	vec3 spotLightCoord;
	bool inSpotLightMap;

	#if defined( USE_SHADOWMAP ) && NUM_SPOT_LIGHT_SHADOWS > 0
	SpotLightShadow spotLightShadow;
	#endif

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHTS; i ++ ) {

		spotLight = spotLights[ i ];

		getSpotLightInfo( spotLight, geometryPosition, directLight );

		// spot lights are ordered [shadows with maps, shadows without maps, maps without shadows, none]
		#if ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#define SPOT_LIGHT_MAP_INDEX UNROLLED_LOOP_INDEX
		#elif ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		#define SPOT_LIGHT_MAP_INDEX NUM_SPOT_LIGHT_MAPS
		#else
		#define SPOT_LIGHT_MAP_INDEX ( UNROLLED_LOOP_INDEX - NUM_SPOT_LIGHT_SHADOWS + NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#endif

		#if ( SPOT_LIGHT_MAP_INDEX < NUM_SPOT_LIGHT_MAPS )
			spotLightCoord = vSpotLightCoord[ i ].xyz / vSpotLightCoord[ i ].w;
			inSpotLightMap = all( lessThan( abs( spotLightCoord * 2. - 1. ), vec3( 1.0 ) ) );
			spotColor = texture2D( spotLightMap[ SPOT_LIGHT_MAP_INDEX ], spotLightCoord.xy );
			directLight.color = inSpotLightMap ? directLight.color * spotColor.rgb : directLight.color;
		#endif

		#undef SPOT_LIGHT_MAP_INDEX

		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		spotLightShadow = spotLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( spotShadowMap[ i ], spotLightShadow.shadowMapSize, spotLightShadow.shadowBias, spotLightShadow.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;
		#endif

		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

	}
	#pragma unroll_loop_end

#endif

#if ( NUM_DIR_LIGHTS > 0 ) && defined( RE_Direct )

	DirectionalLight directionalLight;
	#if defined( USE_SHADOWMAP ) && NUM_DIR_LIGHT_SHADOWS > 0
	DirectionalLightShadow directionalLightShadow;
	#endif

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHTS; i ++ ) {

		directionalLight = directionalLights[ i ];

		getDirectionalLightInfo( directionalLight, directLight );

		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_DIR_LIGHT_SHADOWS )
		directionalLightShadow = directionalLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( directionalShadowMap[ i ], directionalLightShadow.shadowMapSize, directionalLightShadow.shadowBias, directionalLightShadow.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;
		#endif

		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

	}
	#pragma unroll_loop_end

#endif

#if ( NUM_RECT_AREA_LIGHTS > 0 ) && defined( RE_Direct_RectArea )

	RectAreaLight rectAreaLight;

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_RECT_AREA_LIGHTS; i ++ ) {

		rectAreaLight = rectAreaLights[ i ];
		RE_Direct_RectArea( rectAreaLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

	}
	#pragma unroll_loop_end

#endif

#if defined( RE_IndirectDiffuse )

	vec3 iblIrradiance = vec3( 0.0 );

	vec3 irradiance = getAmbientLightIrradiance( ambientLightColor );

	#if defined( USE_LIGHT_PROBES )

		irradiance += getLightProbeIrradiance( lightProbe, geometryNormal );

	#endif

	#if ( NUM_HEMI_LIGHTS > 0 )

		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_HEMI_LIGHTS; i ++ ) {

			irradiance += getHemisphereLightIrradiance( hemisphereLights[ i ], geometryNormal );

		}
		#pragma unroll_loop_end

	#endif

#endif

#if defined( RE_IndirectSpecular )

	vec3 radiance = vec3( 0.0 );
	vec3 clearcoatRadiance = vec3( 0.0 );

#endif
`;var mh=`
#if defined( RE_IndirectDiffuse )

	#ifdef USE_LIGHTMAP

		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		vec3 lightMapIrradiance = lightMapTexel.rgb * lightMapIntensity;

		irradiance += lightMapIrradiance;

	#endif

	#if defined( USE_ENVMAP ) && defined( STANDARD ) && defined( ENVMAP_TYPE_CUBE_UV )

		iblIrradiance += getIBLIrradiance( geometryNormal );

	#endif

#endif

#if defined( USE_ENVMAP ) && defined( RE_IndirectSpecular )

	#ifdef USE_ANISOTROPY

		radiance += getIBLAnisotropyRadiance( geometryViewDir, geometryNormal, material.roughness, material.anisotropyB, material.anisotropy );

	#else

		radiance += getIBLRadiance( geometryViewDir, geometryNormal, material.roughness );

	#endif

	#ifdef USE_CLEARCOAT

		clearcoatRadiance += getIBLRadiance( geometryViewDir, geometryClearcoatNormal, material.clearcoatRoughness );

	#endif

#endif
`;var gh=`
#if defined( RE_IndirectDiffuse )

	RE_IndirectDiffuse( irradiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

#endif

#if defined( RE_IndirectSpecular )

	RE_IndirectSpecular( radiance, iblIrradiance, clearcoatRadiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );

#endif
`;var xh=`
#if defined( USE_LOGDEPTHBUF )

	// Doing a strict comparison with == 1.0 can cause noise artifacts
	// on some platforms. See issue #17623.
	gl_FragDepth = vIsPerspective == 0.0 ? gl_FragCoord.z : log2( vFragDepth ) * logDepthBufFC * 0.5;

#endif
`;var vh=`
#if defined( USE_LOGDEPTHBUF )

	uniform float logDepthBufFC;
	varying float vFragDepth;
	varying float vIsPerspective;

#endif
`;var _h=`
#ifdef USE_LOGDEPTHBUF

	varying float vFragDepth;
	varying float vIsPerspective;

#endif
`;var yh=`
#ifdef USE_LOGDEPTHBUF

	vFragDepth = 1.0 + gl_Position.w;
	vIsPerspective = float( isPerspectiveMatrix( projectionMatrix ) );

#endif
`;var Sh=`
#ifdef USE_MAP

	vec4 sampledDiffuseColor = texture2D( map, vMapUv );

	#ifdef DECODE_VIDEO_TEXTURE

		// use inline sRGB decode until browsers properly support SRGB8_ALPHA8 with video textures (#26516)

		sampledDiffuseColor = vec4( mix( pow( sampledDiffuseColor.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), sampledDiffuseColor.rgb * 0.0773993808, vec3( lessThanEqual( sampledDiffuseColor.rgb, vec3( 0.04045 ) ) ) ), sampledDiffuseColor.w );
	
	#endif

	diffuseColor *= sampledDiffuseColor;

#endif
`;var bh=`
#ifdef USE_MAP

	uniform sampler2D map;

#endif
`;var Rh=`
#if defined( USE_MAP ) || defined( USE_ALPHAMAP )

	#if defined( USE_POINTS_UV )

		vec2 uv = vUv;

	#else

		vec2 uv = ( uvTransform * vec3( gl_PointCoord.x, 1.0 - gl_PointCoord.y, 1 ) ).xy;

	#endif

#endif

#ifdef USE_MAP

	diffuseColor *= texture2D( map, uv );

#endif

#ifdef USE_ALPHAMAP

	diffuseColor.a *= texture2D( alphaMap, uv ).g;

#endif
`;var Mh=`
#if defined( USE_POINTS_UV )

	varying vec2 vUv;

#else

	#if defined( USE_MAP ) || defined( USE_ALPHAMAP )

		uniform mat3 uvTransform;

	#endif

#endif

#ifdef USE_MAP

	uniform sampler2D map;

#endif

#ifdef USE_ALPHAMAP

	uniform sampler2D alphaMap;

#endif
`;var wh=`
float metalnessFactor = metalness;

#ifdef USE_METALNESSMAP

	vec4 texelMetalness = texture2D( metalnessMap, vMetalnessMapUv );

	// reads channel B, compatible with a combined OcclusionRoughnessMetallic (RGB) texture
	metalnessFactor *= texelMetalness.b;

#endif
`;var Th=`
#ifdef USE_METALNESSMAP

	uniform sampler2D metalnessMap;

#endif
`;var Eh=`
#ifdef USE_INSTANCING_MORPH

	float morphTargetInfluences[MORPHTARGETS_COUNT];

	float morphTargetBaseInfluence = texelFetch( morphTexture, ivec2( 0, gl_InstanceID ), 0 ).r;

	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {

		morphTargetInfluences[i] =  texelFetch( morphTexture, ivec2( i + 1, gl_InstanceID ), 0 ).r;

	}
#endif
`;var Ah=`
#if defined( USE_MORPHCOLORS ) && defined( MORPHTARGETS_TEXTURE )

	// morphTargetBaseInfluence is set based on BufferGeometry.morphTargetsRelative value:
	// When morphTargetsRelative is false, this is set to 1 - sum(influences); this results in normal = sum((target - base) * influence)
	// When morphTargetsRelative is true, this is set to 1; as a result, all morph targets are simply added to the base after weighting
	vColor *= morphTargetBaseInfluence;

	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {

		#if defined( USE_COLOR_ALPHA )

			if ( morphTargetInfluences[ i ] != 0.0 ) vColor += getMorph( gl_VertexID, i, 2 ) * morphTargetInfluences[ i ];

		#elif defined( USE_COLOR )

			if ( morphTargetInfluences[ i ] != 0.0 ) vColor += getMorph( gl_VertexID, i, 2 ).rgb * morphTargetInfluences[ i ];

		#endif

	}

#endif
`;var Ch=`
#ifdef USE_MORPHNORMALS

	// morphTargetBaseInfluence is set based on BufferGeometry.morphTargetsRelative value:
	// When morphTargetsRelative is false, this is set to 1 - sum(influences); this results in normal = sum((target - base) * influence)
	// When morphTargetsRelative is true, this is set to 1; as a result, all morph targets are simply added to the base after weighting
	objectNormal *= morphTargetBaseInfluence;

	#ifdef MORPHTARGETS_TEXTURE

		for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {

			if ( morphTargetInfluences[ i ] != 0.0 ) objectNormal += getMorph( gl_VertexID, i, 1 ).xyz * morphTargetInfluences[ i ];

		}

	#else

		objectNormal += morphNormal0 * morphTargetInfluences[ 0 ];
		objectNormal += morphNormal1 * morphTargetInfluences[ 1 ];
		objectNormal += morphNormal2 * morphTargetInfluences[ 2 ];
		objectNormal += morphNormal3 * morphTargetInfluences[ 3 ];

	#endif

#endif
`;var Ph=`
#ifdef USE_MORPHTARGETS

	#ifndef USE_INSTANCING_MORPH

		uniform float morphTargetBaseInfluence;

	#endif

	#ifdef MORPHTARGETS_TEXTURE

		#ifndef USE_INSTANCING_MORPH

			uniform float morphTargetInfluences[ MORPHTARGETS_COUNT ];

		#endif

		uniform sampler2DArray morphTargetsTexture;
		uniform ivec2 morphTargetsTextureSize;

		vec4 getMorph( const in int vertexIndex, const in int morphTargetIndex, const in int offset ) {

			int texelIndex = vertexIndex * MORPHTARGETS_TEXTURE_STRIDE + offset;
			int y = texelIndex / morphTargetsTextureSize.x;
			int x = texelIndex - y * morphTargetsTextureSize.x;

			ivec3 morphUV = ivec3( x, y, morphTargetIndex );
			return texelFetch( morphTargetsTexture, morphUV, 0 );

		}

	#else

		#ifndef USE_MORPHNORMALS

			uniform float morphTargetInfluences[ 8 ];

		#else

			uniform float morphTargetInfluences[ 4 ];

		#endif

	#endif

#endif
`;var Ih=`
#ifdef USE_MORPHTARGETS

	// morphTargetBaseInfluence is set based on BufferGeometry.morphTargetsRelative value:
	// When morphTargetsRelative is false, this is set to 1 - sum(influences); this results in position = sum((target - base) * influence)
	// When morphTargetsRelative is true, this is set to 1; as a result, all morph targets are simply added to the base after weighting
	transformed *= morphTargetBaseInfluence;

	#ifdef MORPHTARGETS_TEXTURE

		for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {

			if ( morphTargetInfluences[ i ] != 0.0 ) transformed += getMorph( gl_VertexID, i, 0 ).xyz * morphTargetInfluences[ i ];

		}

	#else

		transformed += morphTarget0 * morphTargetInfluences[ 0 ];
		transformed += morphTarget1 * morphTargetInfluences[ 1 ];
		transformed += morphTarget2 * morphTargetInfluences[ 2 ];
		transformed += morphTarget3 * morphTargetInfluences[ 3 ];

		#ifndef USE_MORPHNORMALS

			transformed += morphTarget4 * morphTargetInfluences[ 4 ];
			transformed += morphTarget5 * morphTargetInfluences[ 5 ];
			transformed += morphTarget6 * morphTargetInfluences[ 6 ];
			transformed += morphTarget7 * morphTargetInfluences[ 7 ];

		#endif

	#endif

#endif
`;var Dh=`
float faceDirection = gl_FrontFacing ? 1.0 : - 1.0;

#ifdef FLAT_SHADED

	vec3 fdx = dFdx( vViewPosition );
	vec3 fdy = dFdy( vViewPosition );
	vec3 normal = normalize( cross( fdx, fdy ) );

#else

	vec3 normal = normalize( vNormal );

	#ifdef DOUBLE_SIDED

		normal *= faceDirection;

	#endif

#endif

#if defined( USE_NORMALMAP_TANGENTSPACE ) || defined( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY )

	#ifdef USE_TANGENT

		mat3 tbn = mat3( normalize( vTangent ), normalize( vBitangent ), normal );

	#else

		mat3 tbn = getTangentFrame( - vViewPosition, normal,
		#if defined( USE_NORMALMAP )
			vNormalMapUv
		#elif defined( USE_CLEARCOAT_NORMALMAP )
			vClearcoatNormalMapUv
		#else
			vUv
		#endif
		);

	#endif

	#if defined( DOUBLE_SIDED ) && ! defined( FLAT_SHADED )

		tbn[0] *= faceDirection;
		tbn[1] *= faceDirection;

	#endif

#endif

#ifdef USE_CLEARCOAT_NORMALMAP

	#ifdef USE_TANGENT

		mat3 tbn2 = mat3( normalize( vTangent ), normalize( vBitangent ), normal );

	#else

		mat3 tbn2 = getTangentFrame( - vViewPosition, normal, vClearcoatNormalMapUv );

	#endif

	#if defined( DOUBLE_SIDED ) && ! defined( FLAT_SHADED )

		tbn2[0] *= faceDirection;
		tbn2[1] *= faceDirection;

	#endif

#endif

// non perturbed normal for clearcoat among others

vec3 nonPerturbedNormal = normal;

`;var Lh=`

#ifdef USE_NORMALMAP_OBJECTSPACE

	normal = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0; // overrides both flatShading and attribute normals

	#ifdef FLIP_SIDED

		normal = - normal;

	#endif

	#ifdef DOUBLE_SIDED

		normal = normal * faceDirection;

	#endif

	normal = normalize( normalMatrix * normal );

#elif defined( USE_NORMALMAP_TANGENTSPACE )

	vec3 mapN = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
	mapN.xy *= normalScale;

	normal = normalize( tbn * mapN );

#elif defined( USE_BUMPMAP )

	normal = perturbNormalArb( - vViewPosition, normal, dHdxy_fwd(), faceDirection );

#endif
`;var Uh=`
#ifndef FLAT_SHADED

	varying vec3 vNormal;

	#ifdef USE_TANGENT

		varying vec3 vTangent;
		varying vec3 vBitangent;

	#endif

#endif
`;var Fh=`
#ifndef FLAT_SHADED

	varying vec3 vNormal;

	#ifdef USE_TANGENT

		varying vec3 vTangent;
		varying vec3 vBitangent;

	#endif

#endif
`;var Oh=`
#ifndef FLAT_SHADED // normal is computed with derivatives when FLAT_SHADED

	vNormal = normalize( transformedNormal );

	#ifdef USE_TANGENT

		vTangent = normalize( transformedTangent );
		vBitangent = normalize( cross( vNormal, vTangent ) * tangent.w );

	#endif

#endif
`;var Nh=`
#ifdef USE_NORMALMAP

	uniform sampler2D normalMap;
	uniform vec2 normalScale;

#endif

#ifdef USE_NORMALMAP_OBJECTSPACE

	uniform mat3 normalMatrix;

#endif

#if ! defined ( USE_TANGENT ) && ( defined ( USE_NORMALMAP_TANGENTSPACE ) || defined ( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY ) )

	// Normal Mapping Without Precomputed Tangents
	// http://www.thetenthplanet.de/archives/1180

	mat3 getTangentFrame( vec3 eye_pos, vec3 surf_norm, vec2 uv ) {

		vec3 q0 = dFdx( eye_pos.xyz );
		vec3 q1 = dFdy( eye_pos.xyz );
		vec2 st0 = dFdx( uv.st );
		vec2 st1 = dFdy( uv.st );

		vec3 N = surf_norm; // normalized

		vec3 q1perp = cross( q1, N );
		vec3 q0perp = cross( N, q0 );

		vec3 T = q1perp * st0.x + q0perp * st1.x;
		vec3 B = q1perp * st0.y + q0perp * st1.y;

		float det = max( dot( T, T ), dot( B, B ) );
		float scale = ( det == 0.0 ) ? 0.0 : inversesqrt( det );

		return mat3( T * scale, B * scale, N );

	}

#endif
`;var Bh=`
#ifdef USE_CLEARCOAT

	vec3 clearcoatNormal = nonPerturbedNormal;

#endif
`;var kh=`
#ifdef USE_CLEARCOAT_NORMALMAP

	vec3 clearcoatMapN = texture2D( clearcoatNormalMap, vClearcoatNormalMapUv ).xyz * 2.0 - 1.0;
	clearcoatMapN.xy *= clearcoatNormalScale;

	clearcoatNormal = normalize( tbn2 * clearcoatMapN );

#endif
`;var zh=`

#ifdef USE_CLEARCOATMAP

	uniform sampler2D clearcoatMap;

#endif

#ifdef USE_CLEARCOAT_NORMALMAP

	uniform sampler2D clearcoatNormalMap;
	uniform vec2 clearcoatNormalScale;

#endif

#ifdef USE_CLEARCOAT_ROUGHNESSMAP

	uniform sampler2D clearcoatRoughnessMap;

#endif
`;var Vh=`

#ifdef USE_IRIDESCENCEMAP

	uniform sampler2D iridescenceMap;

#endif

#ifdef USE_IRIDESCENCE_THICKNESSMAP

	uniform sampler2D iridescenceThicknessMap;

#endif
`;var Gh=`
#ifdef OPAQUE
diffuseColor.a = 1.0;
#endif

#ifdef USE_TRANSMISSION
diffuseColor.a *= material.transmissionAlpha;
#endif

gl_FragColor = vec4( outgoingLight, diffuseColor.a );
`;var Wh=`
vec3 packNormalToRGB( const in vec3 normal ) {
	return normalize( normal ) * 0.5 + 0.5;
}

vec3 unpackRGBToNormal( const in vec3 rgb ) {
	return 2.0 * rgb.xyz - 1.0;
}

const float PackUpscale = 256. / 255.; // fraction -> 0..1 (including 1)
const float UnpackDownscale = 255. / 256.; // 0..1 -> fraction (excluding 1)

const vec3 PackFactors = vec3( 256. * 256. * 256., 256. * 256., 256. );
const vec4 UnpackFactors = UnpackDownscale / vec4( PackFactors, 1. );

const float ShiftRight8 = 1. / 256.;

vec4 packDepthToRGBA( const in float v ) {
	vec4 r = vec4( fract( v * PackFactors ), v );
	r.yzw -= r.xyz * ShiftRight8; // tidy overflow
	return r * PackUpscale;
}

float unpackRGBAToDepth( const in vec4 v ) {
	return dot( v, UnpackFactors );
}

vec2 packDepthToRG( in highp float v ) {
	return packDepthToRGBA( v ).yx;
}

float unpackRGToDepth( const in highp vec2 v ) {
	return unpackRGBAToDepth( vec4( v.xy, 0.0, 0.0 ) );
}

vec4 pack2HalfToRGBA( vec2 v ) {
	vec4 r = vec4( v.x, fract( v.x * 255.0 ), v.y, fract( v.y * 255.0 ) );
	return vec4( r.x - r.y / 255.0, r.y, r.z - r.w / 255.0, r.w );
}

vec2 unpackRGBATo2Half( vec4 v ) {
	return vec2( v.x + ( v.y / 255.0 ), v.z + ( v.w / 255.0 ) );
}

// NOTE: viewZ, the z-coordinate in camera space, is negative for points in front of the camera

float viewZToOrthographicDepth( const in float viewZ, const in float near, const in float far ) {
	// -near maps to 0; -far maps to 1
	return ( viewZ + near ) / ( near - far );
}

float orthographicDepthToViewZ( const in float depth, const in float near, const in float far ) {
	// maps orthographic depth in [ 0, 1 ] to viewZ
	return depth * ( near - far ) - near;
}

// NOTE: https://twitter.com/gonnavis/status/1377183786949959682

float viewZToPerspectiveDepth( const in float viewZ, const in float near, const in float far ) {
	// -near maps to 0; -far maps to 1
	return ( ( near + viewZ ) * far ) / ( ( far - near ) * viewZ );
}

float perspectiveDepthToViewZ( const in float depth, const in float near, const in float far ) {
	// maps perspective depth in [ 0, 1 ] to viewZ
	return ( near * far ) / ( ( far - near ) * depth - far );
}
`;var Hh=`
#ifdef PREMULTIPLIED_ALPHA

	// Get get normal blending with premultipled, use with CustomBlending, OneFactor, OneMinusSrcAlphaFactor, AddEquation.
	gl_FragColor.rgb *= gl_FragColor.a;

#endif
`;var jh=`
vec4 mvPosition = vec4( transformed, 1.0 );

#ifdef USE_BATCHING

	mvPosition = batchingMatrix * mvPosition;

#endif

#ifdef USE_INSTANCING

	mvPosition = instanceMatrix * mvPosition;

#endif

mvPosition = modelViewMatrix * mvPosition;

gl_Position = projectionMatrix * mvPosition;
`;var Xh=`
#ifdef DITHERING

	gl_FragColor.rgb = dithering( gl_FragColor.rgb );

#endif
`;var qh=`
#ifdef DITHERING

	// based on https://www.shadertoy.com/view/MslGR8
	vec3 dithering( vec3 color ) {
		//Calculate grid position
		float grid_position = rand( gl_FragCoord.xy );

		//Shift the individual colors differently, thus making it even harder to see the dithering pattern
		vec3 dither_shift_RGB = vec3( 0.25 / 255.0, -0.25 / 255.0, 0.25 / 255.0 );

		//modify shift according to grid position.
		dither_shift_RGB = mix( 2.0 * dither_shift_RGB, -2.0 * dither_shift_RGB, grid_position );

		//shift the color by dither_shift
		return color + dither_shift_RGB;
	}

#endif
`;var $h=`
float roughnessFactor = roughness;

#ifdef USE_ROUGHNESSMAP

	vec4 texelRoughness = texture2D( roughnessMap, vRoughnessMapUv );

	// reads channel G, compatible with a combined OcclusionRoughnessMetallic (RGB) texture
	roughnessFactor *= texelRoughness.g;

#endif
`;var Yh=`
#ifdef USE_ROUGHNESSMAP

	uniform sampler2D roughnessMap;

#endif
`;var Kh=`
#if NUM_SPOT_LIGHT_COORDS > 0

	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];

#endif

#if NUM_SPOT_LIGHT_MAPS > 0

	uniform sampler2D spotLightMap[ NUM_SPOT_LIGHT_MAPS ];

#endif

#ifdef USE_SHADOWMAP

	#if NUM_DIR_LIGHT_SHADOWS > 0

		uniform sampler2D directionalShadowMap[ NUM_DIR_LIGHT_SHADOWS ];
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];

		struct DirectionalLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};

		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];

	#endif

	#if NUM_SPOT_LIGHT_SHADOWS > 0

		uniform sampler2D spotShadowMap[ NUM_SPOT_LIGHT_SHADOWS ];

		struct SpotLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};

		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];

	#endif

	#if NUM_POINT_LIGHT_SHADOWS > 0

		uniform sampler2D pointShadowMap[ NUM_POINT_LIGHT_SHADOWS ];
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];

		struct PointLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};

		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];

	#endif

	/*
	#if NUM_RECT_AREA_LIGHTS > 0

		// TODO (abelnation): create uniforms for area light shadows

	#endif
	*/

	float texture2DCompare( sampler2D depths, vec2 uv, float compare ) {

		return step( compare, unpackRGBAToDepth( texture2D( depths, uv ) ) );

	}

	vec2 texture2DDistribution( sampler2D shadow, vec2 uv ) {

		return unpackRGBATo2Half( texture2D( shadow, uv ) );

	}

	float VSMShadow (sampler2D shadow, vec2 uv, float compare ){

		float occlusion = 1.0;

		vec2 distribution = texture2DDistribution( shadow, uv );

		float hard_shadow = step( compare , distribution.x ); // Hard Shadow

		if (hard_shadow != 1.0 ) {

			float distance = compare - distribution.x ;
			float variance = max( 0.00000, distribution.y * distribution.y );
			float softness_probability = variance / (variance + distance * distance ); // Chebeyshevs inequality
			softness_probability = clamp( ( softness_probability - 0.3 ) / ( 0.95 - 0.3 ), 0.0, 1.0 ); // 0.3 reduces light bleed
			occlusion = clamp( max( hard_shadow, softness_probability ), 0.0, 1.0 );

		}
		return occlusion;

	}

	float getShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowBias, float shadowRadius, vec4 shadowCoord ) {

		float shadow = 1.0;

		shadowCoord.xyz /= shadowCoord.w;
		shadowCoord.z += shadowBias;

		bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
		bool frustumTest = inFrustum && shadowCoord.z <= 1.0;

		if ( frustumTest ) {

		#if defined( SHADOWMAP_TYPE_PCF )

			vec2 texelSize = vec2( 1.0 ) / shadowMapSize;

			float dx0 = - texelSize.x * shadowRadius;
			float dy0 = - texelSize.y * shadowRadius;
			float dx1 = + texelSize.x * shadowRadius;
			float dy1 = + texelSize.y * shadowRadius;
			float dx2 = dx0 / 2.0;
			float dy2 = dy0 / 2.0;
			float dx3 = dx1 / 2.0;
			float dy3 = dy1 / 2.0;

			shadow = (
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx0, dy0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( 0.0, dy0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx1, dy0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx2, dy2 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( 0.0, dy2 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx3, dy2 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx0, 0.0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx2, 0.0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy, shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx3, 0.0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx1, 0.0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx2, dy3 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( 0.0, dy3 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx3, dy3 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx0, dy1 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( 0.0, dy1 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, shadowCoord.xy + vec2( dx1, dy1 ), shadowCoord.z )
			) * ( 1.0 / 17.0 );

		#elif defined( SHADOWMAP_TYPE_PCF_SOFT )

			vec2 texelSize = vec2( 1.0 ) / shadowMapSize;
			float dx = texelSize.x;
			float dy = texelSize.y;

			vec2 uv = shadowCoord.xy;
			vec2 f = fract( uv * shadowMapSize + 0.5 );
			uv -= f * texelSize;

			shadow = (
				texture2DCompare( shadowMap, uv, shadowCoord.z ) +
				texture2DCompare( shadowMap, uv + vec2( dx, 0.0 ), shadowCoord.z ) +
				texture2DCompare( shadowMap, uv + vec2( 0.0, dy ), shadowCoord.z ) +
				texture2DCompare( shadowMap, uv + texelSize, shadowCoord.z ) +
				mix( texture2DCompare( shadowMap, uv + vec2( -dx, 0.0 ), shadowCoord.z ),
					 texture2DCompare( shadowMap, uv + vec2( 2.0 * dx, 0.0 ), shadowCoord.z ),
					 f.x ) +
				mix( texture2DCompare( shadowMap, uv + vec2( -dx, dy ), shadowCoord.z ),
					 texture2DCompare( shadowMap, uv + vec2( 2.0 * dx, dy ), shadowCoord.z ),
					 f.x ) +
				mix( texture2DCompare( shadowMap, uv + vec2( 0.0, -dy ), shadowCoord.z ),
					 texture2DCompare( shadowMap, uv + vec2( 0.0, 2.0 * dy ), shadowCoord.z ),
					 f.y ) +
				mix( texture2DCompare( shadowMap, uv + vec2( dx, -dy ), shadowCoord.z ),
					 texture2DCompare( shadowMap, uv + vec2( dx, 2.0 * dy ), shadowCoord.z ),
					 f.y ) +
				mix( mix( texture2DCompare( shadowMap, uv + vec2( -dx, -dy ), shadowCoord.z ),
						  texture2DCompare( shadowMap, uv + vec2( 2.0 * dx, -dy ), shadowCoord.z ),
						  f.x ),
					 mix( texture2DCompare( shadowMap, uv + vec2( -dx, 2.0 * dy ), shadowCoord.z ),
						  texture2DCompare( shadowMap, uv + vec2( 2.0 * dx, 2.0 * dy ), shadowCoord.z ),
						  f.x ),
					 f.y )
			) * ( 1.0 / 9.0 );

		#elif defined( SHADOWMAP_TYPE_VSM )

			shadow = VSMShadow( shadowMap, shadowCoord.xy, shadowCoord.z );

		#else // no percentage-closer filtering:

			shadow = texture2DCompare( shadowMap, shadowCoord.xy, shadowCoord.z );

		#endif

		}

		return shadow;

	}

	// cubeToUV() maps a 3D direction vector suitable for cube texture mapping to a 2D
	// vector suitable for 2D texture mapping. This code uses the following layout for the
	// 2D texture:
	//
	// xzXZ
	//  y Y
	//
	// Y - Positive y direction
	// y - Negative y direction
	// X - Positive x direction
	// x - Negative x direction
	// Z - Positive z direction
	// z - Negative z direction
	//
	// Source and test bed:
	// https://gist.github.com/tschw/da10c43c467ce8afd0c4

	vec2 cubeToUV( vec3 v, float texelSizeY ) {

		// Number of texels to avoid at the edge of each square

		vec3 absV = abs( v );

		// Intersect unit cube

		float scaleToCube = 1.0 / max( absV.x, max( absV.y, absV.z ) );
		absV *= scaleToCube;

		// Apply scale to avoid seams

		// two texels less per square (one texel will do for NEAREST)
		v *= scaleToCube * ( 1.0 - 2.0 * texelSizeY );

		// Unwrap

		// space: -1 ... 1 range for each square
		//
		// #X##		dim    := ( 4 , 2 )
		//  # #		center := ( 1 , 1 )

		vec2 planar = v.xy;

		float almostATexel = 1.5 * texelSizeY;
		float almostOne = 1.0 - almostATexel;

		if ( absV.z >= almostOne ) {

			if ( v.z > 0.0 )
				planar.x = 4.0 - v.x;

		} else if ( absV.x >= almostOne ) {

			float signX = sign( v.x );
			planar.x = v.z * signX + 2.0 * signX;

		} else if ( absV.y >= almostOne ) {

			float signY = sign( v.y );
			planar.x = v.x + 2.0 * signY + 2.0;
			planar.y = v.z * signY - 2.0;

		}

		// Transform to UV space

		// scale := 0.5 / dim
		// translate := ( center + 0.5 ) / dim
		return vec2( 0.125, 0.25 ) * planar + vec2( 0.375, 0.75 );

	}

	float getPointShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowBias, float shadowRadius, vec4 shadowCoord, float shadowCameraNear, float shadowCameraFar ) {

		float shadow = 1.0;

		// for point lights, the uniform @vShadowCoord is re-purposed to hold
		// the vector from the light to the world-space position of the fragment.
		vec3 lightToPosition = shadowCoord.xyz;
		
		float lightToPositionLength = length( lightToPosition );

		if ( lightToPositionLength - shadowCameraFar <= 0.0 && lightToPositionLength - shadowCameraNear >= 0.0 ) {

			// dp = normalized distance from light to fragment position
			float dp = ( lightToPositionLength - shadowCameraNear ) / ( shadowCameraFar - shadowCameraNear ); // need to clamp?
			dp += shadowBias;

			// bd3D = base direction 3D
			vec3 bd3D = normalize( lightToPosition );

			vec2 texelSize = vec2( 1.0 ) / ( shadowMapSize * vec2( 4.0, 2.0 ) );

			#if defined( SHADOWMAP_TYPE_PCF ) || defined( SHADOWMAP_TYPE_PCF_SOFT ) || defined( SHADOWMAP_TYPE_VSM )

				vec2 offset = vec2( - 1, 1 ) * shadowRadius * texelSize.y;

				shadow = (
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.xyy, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.yyy, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.xyx, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.yyx, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.xxy, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.yxy, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.xxx, texelSize.y ), dp ) +
					texture2DCompare( shadowMap, cubeToUV( bd3D + offset.yxx, texelSize.y ), dp )
				) * ( 1.0 / 9.0 );

			#else // no percentage-closer filtering

				shadow = texture2DCompare( shadowMap, cubeToUV( bd3D, texelSize.y ), dp );

			#endif

		}

		return shadow;

	}

#endif
`;var Zh=`

#if NUM_SPOT_LIGHT_COORDS > 0

	uniform mat4 spotLightMatrix[ NUM_SPOT_LIGHT_COORDS ];
	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];

#endif

#ifdef USE_SHADOWMAP

	#if NUM_DIR_LIGHT_SHADOWS > 0

		uniform mat4 directionalShadowMatrix[ NUM_DIR_LIGHT_SHADOWS ];
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];

		struct DirectionalLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};

		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];

	#endif

	#if NUM_SPOT_LIGHT_SHADOWS > 0

		struct SpotLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};

		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];

	#endif

	#if NUM_POINT_LIGHT_SHADOWS > 0

		uniform mat4 pointShadowMatrix[ NUM_POINT_LIGHT_SHADOWS ];
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];

		struct PointLightShadow {
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};

		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];

	#endif

	/*
	#if NUM_RECT_AREA_LIGHTS > 0

		// TODO (abelnation): uniforms for area light shadows

	#endif
	*/

#endif
`;var Jh=`

#if ( defined( USE_SHADOWMAP ) && ( NUM_DIR_LIGHT_SHADOWS > 0 || NUM_POINT_LIGHT_SHADOWS > 0 ) ) || ( NUM_SPOT_LIGHT_COORDS > 0 )

	// Offsetting the position used for querying occlusion along the world normal can be used to reduce shadow acne.
	vec3 shadowWorldNormal = inverseTransformDirection( transformedNormal, viewMatrix );
	vec4 shadowWorldPosition;

#endif

#if defined( USE_SHADOWMAP )

	#if NUM_DIR_LIGHT_SHADOWS > 0

		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {

			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * directionalLightShadows[ i ].shadowNormalBias, 0 );
			vDirectionalShadowCoord[ i ] = directionalShadowMatrix[ i ] * shadowWorldPosition;

		}
		#pragma unroll_loop_end

	#endif

	#if NUM_POINT_LIGHT_SHADOWS > 0

		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {

			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * pointLightShadows[ i ].shadowNormalBias, 0 );
			vPointShadowCoord[ i ] = pointShadowMatrix[ i ] * shadowWorldPosition;

		}
		#pragma unroll_loop_end

	#endif

	/*
	#if NUM_RECT_AREA_LIGHTS > 0

		// TODO (abelnation): update vAreaShadowCoord with area light info

	#endif
	*/

#endif

// spot lights can be evaluated without active shadow mapping (when SpotLight.map is used)

#if NUM_SPOT_LIGHT_COORDS > 0

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_COORDS; i ++ ) {

		shadowWorldPosition = worldPosition;
		#if ( defined( USE_SHADOWMAP ) && UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
			shadowWorldPosition.xyz += shadowWorldNormal * spotLightShadows[ i ].shadowNormalBias;
		#endif
		vSpotLightCoord[ i ] = spotLightMatrix[ i ] * shadowWorldPosition;

	}
	#pragma unroll_loop_end

#endif


`;var Qh=`
float getShadowMask() {

	float shadow = 1.0;

	#ifdef USE_SHADOWMAP

	#if NUM_DIR_LIGHT_SHADOWS > 0

	DirectionalLightShadow directionalLight;

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {

		directionalLight = directionalLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( directionalShadowMap[ i ], directionalLight.shadowMapSize, directionalLight.shadowBias, directionalLight.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;

	}
	#pragma unroll_loop_end

	#endif

	#if NUM_SPOT_LIGHT_SHADOWS > 0

	SpotLightShadow spotLight;

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_SHADOWS; i ++ ) {

		spotLight = spotLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( spotShadowMap[ i ], spotLight.shadowMapSize, spotLight.shadowBias, spotLight.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;

	}
	#pragma unroll_loop_end

	#endif

	#if NUM_POINT_LIGHT_SHADOWS > 0

	PointLightShadow pointLight;

	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {

		pointLight = pointLightShadows[ i ];
		shadow *= receiveShadow ? getPointShadow( pointShadowMap[ i ], pointLight.shadowMapSize, pointLight.shadowBias, pointLight.shadowRadius, vPointShadowCoord[ i ], pointLight.shadowCameraNear, pointLight.shadowCameraFar ) : 1.0;

	}
	#pragma unroll_loop_end

	#endif

	/*
	#if NUM_RECT_AREA_LIGHTS > 0

		// TODO (abelnation): update shadow for Area light

	#endif
	*/

	#endif

	return shadow;

}
`;var ef=`
#ifdef USE_SKINNING

	mat4 boneMatX = getBoneMatrix( skinIndex.x );
	mat4 boneMatY = getBoneMatrix( skinIndex.y );
	mat4 boneMatZ = getBoneMatrix( skinIndex.z );
	mat4 boneMatW = getBoneMatrix( skinIndex.w );

#endif
`;var tf=`
#ifdef USE_SKINNING

	uniform mat4 bindMatrix;
	uniform mat4 bindMatrixInverse;

	uniform highp sampler2D boneTexture;

	mat4 getBoneMatrix( const in float i ) {

		int size = textureSize( boneTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( boneTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( boneTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( boneTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( boneTexture, ivec2( x + 3, y ), 0 );

		return mat4( v1, v2, v3, v4 );

	}

#endif
`;var rf=`
#ifdef USE_SKINNING

	vec4 skinVertex = bindMatrix * vec4( transformed, 1.0 );

	vec4 skinned = vec4( 0.0 );
	skinned += boneMatX * skinVertex * skinWeight.x;
	skinned += boneMatY * skinVertex * skinWeight.y;
	skinned += boneMatZ * skinVertex * skinWeight.z;
	skinned += boneMatW * skinVertex * skinWeight.w;

	transformed = ( bindMatrixInverse * skinned ).xyz;

#endif
`;var nf=`
#ifdef USE_SKINNING

	mat4 skinMatrix = mat4( 0.0 );
	skinMatrix += skinWeight.x * boneMatX;
	skinMatrix += skinWeight.y * boneMatY;
	skinMatrix += skinWeight.z * boneMatZ;
	skinMatrix += skinWeight.w * boneMatW;
	skinMatrix = bindMatrixInverse * skinMatrix * bindMatrix;

	objectNormal = vec4( skinMatrix * vec4( objectNormal, 0.0 ) ).xyz;

	#ifdef USE_TANGENT

		objectTangent = vec4( skinMatrix * vec4( objectTangent, 0.0 ) ).xyz;

	#endif

#endif
`;var of=`
float specularStrength;

#ifdef USE_SPECULARMAP

	vec4 texelSpecular = texture2D( specularMap, vSpecularMapUv );
	specularStrength = texelSpecular.r;

#else

	specularStrength = 1.0;

#endif
`;var sf=`
#ifdef USE_SPECULARMAP

	uniform sampler2D specularMap;

#endif
`;var af=`
#if defined( TONE_MAPPING )

	gl_FragColor.rgb = toneMapping( gl_FragColor.rgb );

#endif
`;var cf=`
#ifndef saturate
// <common> may have defined saturate() already
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif

uniform float toneMappingExposure;

// exposure only
vec3 LinearToneMapping( vec3 color ) {

	return saturate( toneMappingExposure * color );

}

// source: https://www.cs.utah.edu/docs/techreports/2002/pdf/UUCS-02-001.pdf
vec3 ReinhardToneMapping( vec3 color ) {

	color *= toneMappingExposure;
	return saturate( color / ( vec3( 1.0 ) + color ) );

}

// source: http://filmicworlds.com/blog/filmic-tonemapping-operators/
vec3 OptimizedCineonToneMapping( vec3 color ) {

	// optimized filmic operator by Jim Hejl and Richard Burgess-Dawson
	color *= toneMappingExposure;
	color = max( vec3( 0.0 ), color - 0.004 );
	return pow( ( color * ( 6.2 * color + 0.5 ) ) / ( color * ( 6.2 * color + 1.7 ) + 0.06 ), vec3( 2.2 ) );

}

// source: https://github.com/selfshadow/ltc_code/blob/master/webgl/shaders/ltc/ltc_blit.fs
vec3 RRTAndODTFit( vec3 v ) {

	vec3 a = v * ( v + 0.0245786 ) - 0.000090537;
	vec3 b = v * ( 0.983729 * v + 0.4329510 ) + 0.238081;
	return a / b;

}

// this implementation of ACES is modified to accommodate a brighter viewing environment.
// the scale factor of 1/0.6 is subjective. see discussion in #19621.

vec3 ACESFilmicToneMapping( vec3 color ) {

	// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
	const mat3 ACESInputMat = mat3(
		vec3( 0.59719, 0.07600, 0.02840 ), // transposed from source
		vec3( 0.35458, 0.90834, 0.13383 ),
		vec3( 0.04823, 0.01566, 0.83777 )
	);

	// ODT_SAT => XYZ => D60_2_D65 => sRGB
	const mat3 ACESOutputMat = mat3(
		vec3(  1.60475, -0.10208, -0.00327 ), // transposed from source
		vec3( -0.53108,  1.10813, -0.07276 ),
		vec3( -0.07367, -0.00605,  1.07602 )
	);

	color *= toneMappingExposure / 0.6;

	color = ACESInputMat * color;

	// Apply RRT and ODT
	color = RRTAndODTFit( color );

	color = ACESOutputMat * color;

	// Clamp to [0, 1]
	return saturate( color );

}

// Matrices for rec 2020 <> rec 709 color space conversion
// matrix provided in row-major order so it has been transposed
// https://www.itu.int/pub/R-REP-BT.2407-2017
const mat3 LINEAR_REC2020_TO_LINEAR_SRGB = mat3(
	vec3( 1.6605, - 0.1246, - 0.0182 ),
	vec3( - 0.5876, 1.1329, - 0.1006 ),
	vec3( - 0.0728, - 0.0083, 1.1187 )
);

const mat3 LINEAR_SRGB_TO_LINEAR_REC2020 = mat3(
	vec3( 0.6274, 0.0691, 0.0164 ),
	vec3( 0.3293, 0.9195, 0.0880 ),
	vec3( 0.0433, 0.0113, 0.8956 )
);

// https://iolite-engine.com/blog_posts/minimal_agx_implementation
// Mean error^2: 3.6705141e-06
vec3 agxDefaultContrastApprox( vec3 x ) {

	vec3 x2 = x * x;
	vec3 x4 = x2 * x2;

	return + 15.5 * x4 * x2
		- 40.14 * x4 * x
		+ 31.96 * x4
		- 6.868 * x2 * x
		+ 0.4298 * x2
		+ 0.1191 * x
		- 0.00232;

}

// AgX Tone Mapping implementation based on Filament, which in turn is based
// on Blender's implementation using rec 2020 primaries
// https://github.com/google/filament/pull/7236
// Inputs and outputs are encoded as Linear-sRGB.

vec3 AgXToneMapping( vec3 color ) {

	// AgX constants
	const mat3 AgXInsetMatrix = mat3(
		vec3( 0.856627153315983, 0.137318972929847, 0.11189821299995 ),
		vec3( 0.0951212405381588, 0.761241990602591, 0.0767994186031903 ),
		vec3( 0.0482516061458583, 0.101439036467562, 0.811302368396859 )
	);

	// explicit AgXOutsetMatrix generated from Filaments AgXOutsetMatrixInv
	const mat3 AgXOutsetMatrix = mat3(
		vec3( 1.1271005818144368, - 0.1413297634984383, - 0.14132976349843826 ),
		vec3( - 0.11060664309660323, 1.157823702216272, - 0.11060664309660294 ),
		vec3( - 0.016493938717834573, - 0.016493938717834257, 1.2519364065950405 )
	);

	// LOG2_MIN      = -10.0
	// LOG2_MAX      =  +6.5
	// MIDDLE_GRAY   =  0.18
	const float AgxMinEv = - 12.47393;  // log2( pow( 2, LOG2_MIN ) * MIDDLE_GRAY )
	const float AgxMaxEv = 4.026069;    // log2( pow( 2, LOG2_MAX ) * MIDDLE_GRAY )

	color *= toneMappingExposure;

	color = LINEAR_SRGB_TO_LINEAR_REC2020 * color;

	color = AgXInsetMatrix * color;

	// Log2 encoding
	color = max( color, 1e-10 ); // avoid 0 or negative numbers for log2
	color = log2( color );
	color = ( color - AgxMinEv ) / ( AgxMaxEv - AgxMinEv );

	color = clamp( color, 0.0, 1.0 );

	// Apply sigmoid
	color = agxDefaultContrastApprox( color );

	// Apply AgX look
	// v = agxLook(v, look);

	color = AgXOutsetMatrix * color;

	// Linearize
	color = pow( max( vec3( 0.0 ), color ), vec3( 2.2 ) );

	color = LINEAR_REC2020_TO_LINEAR_SRGB * color;

	// Gamut mapping. Simple clamp for now.
	color = clamp( color, 0.0, 1.0 );

	return color;

}

// https://modelviewer.dev/examples/tone-mapping

vec3 NeutralToneMapping( vec3 color ) {

	const float StartCompression = 0.8 - 0.04;
	const float Desaturation = 0.15;

	color *= toneMappingExposure;

	float x = min( color.r, min( color.g, color.b ) );

	float offset = x < 0.08 ? x - 6.25 * x * x : 0.04;

	color -= offset;

	float peak = max( color.r, max( color.g, color.b ) );

	if ( peak < StartCompression ) return color;

	float d = 1. - StartCompression;

	float newPeak = 1. - d * d / ( peak + d - StartCompression );

	color *= newPeak / peak;

	float g = 1. - 1. / ( Desaturation * ( peak - newPeak ) + 1. );

	return mix( color, vec3( newPeak ), g );

}

vec3 CustomToneMapping( vec3 color ) { return color; }
`;var lf=`
#ifdef USE_TRANSMISSION

	material.transmission = transmission;
	material.transmissionAlpha = 1.0;
	material.thickness = thickness;
	material.attenuationDistance = attenuationDistance;
	material.attenuationColor = attenuationColor;

	#ifdef USE_TRANSMISSIONMAP

		material.transmission *= texture2D( transmissionMap, vTransmissionMapUv ).r;

	#endif

	#ifdef USE_THICKNESSMAP

		material.thickness *= texture2D( thicknessMap, vThicknessMapUv ).g;

	#endif

	vec3 pos = vWorldPosition;
	vec3 v = normalize( cameraPosition - pos );
	vec3 n = inverseTransformDirection( normal, viewMatrix );

	vec4 transmitted = getIBLVolumeRefraction(
		n, v, material.roughness, material.diffuseColor, material.specularColor, material.specularF90,
		pos, modelMatrix, viewMatrix, projectionMatrix, material.dispersion, material.ior, material.thickness,
		material.attenuationColor, material.attenuationDistance );

	material.transmissionAlpha = mix( material.transmissionAlpha, transmitted.a, material.transmission );

	totalDiffuse = mix( totalDiffuse, transmitted.rgb, material.transmission );

#endif
`;var df=`
#ifdef USE_TRANSMISSION

	// Transmission code is based on glTF-Sampler-Viewer
	// https://github.com/KhronosGroup/glTF-Sample-Viewer

	uniform float transmission;
	uniform float thickness;
	uniform float attenuationDistance;
	uniform vec3 attenuationColor;

	#ifdef USE_TRANSMISSIONMAP

		uniform sampler2D transmissionMap;

	#endif

	#ifdef USE_THICKNESSMAP

		uniform sampler2D thicknessMap;

	#endif

	uniform vec2 transmissionSamplerSize;
	uniform sampler2D transmissionSamplerMap;

	uniform mat4 modelMatrix;
	uniform mat4 projectionMatrix;

	varying vec3 vWorldPosition;

	// Mipped Bicubic Texture Filtering by N8
	// https://www.shadertoy.com/view/Dl2SDW

	float w0( float a ) {

		return ( 1.0 / 6.0 ) * ( a * ( a * ( - a + 3.0 ) - 3.0 ) + 1.0 );

	}

	float w1( float a ) {

		return ( 1.0 / 6.0 ) * ( a *  a * ( 3.0 * a - 6.0 ) + 4.0 );

	}

	float w2( float a ){

		return ( 1.0 / 6.0 ) * ( a * ( a * ( - 3.0 * a + 3.0 ) + 3.0 ) + 1.0 );

	}

	float w3( float a ) {

		return ( 1.0 / 6.0 ) * ( a * a * a );

	}

	// g0 and g1 are the two amplitude functions
	float g0( float a ) {

		return w0( a ) + w1( a );

	}

	float g1( float a ) {

		return w2( a ) + w3( a );

	}

	// h0 and h1 are the two offset functions
	float h0( float a ) {

		return - 1.0 + w1( a ) / ( w0( a ) + w1( a ) );

	}

	float h1( float a ) {

		return 1.0 + w3( a ) / ( w2( a ) + w3( a ) );

	}

	vec4 bicubic( sampler2D tex, vec2 uv, vec4 texelSize, float lod ) {

		uv = uv * texelSize.zw + 0.5;

		vec2 iuv = floor( uv );
		vec2 fuv = fract( uv );

		float g0x = g0( fuv.x );
		float g1x = g1( fuv.x );
		float h0x = h0( fuv.x );
		float h1x = h1( fuv.x );
		float h0y = h0( fuv.y );
		float h1y = h1( fuv.y );

		vec2 p0 = ( vec2( iuv.x + h0x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p1 = ( vec2( iuv.x + h1x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p2 = ( vec2( iuv.x + h0x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;
		vec2 p3 = ( vec2( iuv.x + h1x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;

		return g0( fuv.y ) * ( g0x * textureLod( tex, p0, lod ) + g1x * textureLod( tex, p1, lod ) ) +
			g1( fuv.y ) * ( g0x * textureLod( tex, p2, lod ) + g1x * textureLod( tex, p3, lod ) );

	}

	vec4 textureBicubic( sampler2D sampler, vec2 uv, float lod ) {

		vec2 fLodSize = vec2( textureSize( sampler, int( lod ) ) );
		vec2 cLodSize = vec2( textureSize( sampler, int( lod + 1.0 ) ) );
		vec2 fLodSizeInv = 1.0 / fLodSize;
		vec2 cLodSizeInv = 1.0 / cLodSize;
		vec4 fSample = bicubic( sampler, uv, vec4( fLodSizeInv, fLodSize ), floor( lod ) );
		vec4 cSample = bicubic( sampler, uv, vec4( cLodSizeInv, cLodSize ), ceil( lod ) );
		return mix( fSample, cSample, fract( lod ) );

	}

	vec3 getVolumeTransmissionRay( const in vec3 n, const in vec3 v, const in float thickness, const in float ior, const in mat4 modelMatrix ) {

		// Direction of refracted light.
		vec3 refractionVector = refract( - v, normalize( n ), 1.0 / ior );

		// Compute rotation-independant scaling of the model matrix.
		vec3 modelScale;
		modelScale.x = length( vec3( modelMatrix[ 0 ].xyz ) );
		modelScale.y = length( vec3( modelMatrix[ 1 ].xyz ) );
		modelScale.z = length( vec3( modelMatrix[ 2 ].xyz ) );

		// The thickness is specified in local space.
		return normalize( refractionVector ) * thickness * modelScale;

	}

	float applyIorToRoughness( const in float roughness, const in float ior ) {

		// Scale roughness with IOR so that an IOR of 1.0 results in no microfacet refraction and
		// an IOR of 1.5 results in the default amount of microfacet refraction.
		return roughness * clamp( ior * 2.0 - 2.0, 0.0, 1.0 );

	}

	vec4 getTransmissionSample( const in vec2 fragCoord, const in float roughness, const in float ior ) {

		float lod = log2( transmissionSamplerSize.x ) * applyIorToRoughness( roughness, ior );
		return textureBicubic( transmissionSamplerMap, fragCoord.xy, lod );

	}

	vec3 volumeAttenuation( const in float transmissionDistance, const in vec3 attenuationColor, const in float attenuationDistance ) {

		if ( isinf( attenuationDistance ) ) {

			// Attenuation distance is +\u221E, i.e. the transmitted color is not attenuated at all.
			return vec3( 1.0 );

		} else {

			// Compute light attenuation using Beer's law.
			vec3 attenuationCoefficient = -log( attenuationColor ) / attenuationDistance;
			vec3 transmittance = exp( - attenuationCoefficient * transmissionDistance ); // Beer's law
			return transmittance;

		}

	}

	vec4 getIBLVolumeRefraction( const in vec3 n, const in vec3 v, const in float roughness, const in vec3 diffuseColor,
		const in vec3 specularColor, const in float specularF90, const in vec3 position, const in mat4 modelMatrix,
		const in mat4 viewMatrix, const in mat4 projMatrix, const in float dispersion, const in float ior, const in float thickness,
		const in vec3 attenuationColor, const in float attenuationDistance ) {

		vec4 transmittedLight;
		vec3 transmittance;

		#ifdef USE_DISPERSION

			float halfSpread = ( ior - 1.0 ) * 0.025 * dispersion;
			vec3 iors = vec3( ior - halfSpread, ior, ior + halfSpread );

			for ( int i = 0; i < 3; i ++ ) {

				vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, iors[ i ], modelMatrix );
				vec3 refractedRayExit = position + transmissionRay;
		
				// Project refracted vector on the framebuffer, while mapping to normalized device coordinates.
				vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
				vec2 refractionCoords = ndcPos.xy / ndcPos.w;
				refractionCoords += 1.0;
				refractionCoords /= 2.0;
		
				// Sample framebuffer to get pixel the refracted ray hits.
				vec4 transmissionSample = getTransmissionSample( refractionCoords, roughness, iors[ i ] );
				transmittedLight[ i ] = transmissionSample[ i ];
				transmittedLight.a += transmissionSample.a;

				transmittance[ i ] = diffuseColor[ i ] * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance )[ i ];

			}

			transmittedLight.a /= 3.0;
		
		#else
		
			vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, ior, modelMatrix );
			vec3 refractedRayExit = position + transmissionRay;

			// Project refracted vector on the framebuffer, while mapping to normalized device coordinates.
			vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
			vec2 refractionCoords = ndcPos.xy / ndcPos.w;
			refractionCoords += 1.0;
			refractionCoords /= 2.0;

			// Sample framebuffer to get pixel the refracted ray hits.
			transmittedLight = getTransmissionSample( refractionCoords, roughness, ior );
			transmittance = diffuseColor * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance );
		
		#endif

		vec3 attenuatedColor = transmittance * transmittedLight.rgb;

		// Get the specular component.
		vec3 F = EnvironmentBRDF( n, v, specularColor, specularF90, roughness );

		// As less light is transmitted, the opacity should be increased. This simple approximation does a decent job 
		// of modulating a CSS background, and has no effect when the buffer is opaque, due to a solid object or clear color.
		float transmittanceFactor = ( transmittance.r + transmittance.g + transmittance.b ) / 3.0;

		return vec4( ( 1.0 - F ) * attenuatedColor, 1.0 - ( 1.0 - transmittedLight.a ) * transmittanceFactor );

	}
#endif
`;var uf=`
#if defined( USE_UV ) || defined( USE_ANISOTROPY )

	varying vec2 vUv;

#endif
#ifdef USE_MAP

	varying vec2 vMapUv;

#endif
#ifdef USE_ALPHAMAP

	varying vec2 vAlphaMapUv;

#endif
#ifdef USE_LIGHTMAP

	varying vec2 vLightMapUv;

#endif
#ifdef USE_AOMAP

	varying vec2 vAoMapUv;

#endif
#ifdef USE_BUMPMAP

	varying vec2 vBumpMapUv;

#endif
#ifdef USE_NORMALMAP

	varying vec2 vNormalMapUv;

#endif
#ifdef USE_EMISSIVEMAP

	varying vec2 vEmissiveMapUv;

#endif
#ifdef USE_METALNESSMAP

	varying vec2 vMetalnessMapUv;

#endif
#ifdef USE_ROUGHNESSMAP

	varying vec2 vRoughnessMapUv;

#endif
#ifdef USE_ANISOTROPYMAP

	varying vec2 vAnisotropyMapUv;

#endif
#ifdef USE_CLEARCOATMAP

	varying vec2 vClearcoatMapUv;

#endif
#ifdef USE_CLEARCOAT_NORMALMAP

	varying vec2 vClearcoatNormalMapUv;

#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP

	varying vec2 vClearcoatRoughnessMapUv;

#endif
#ifdef USE_IRIDESCENCEMAP

	varying vec2 vIridescenceMapUv;

#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP

	varying vec2 vIridescenceThicknessMapUv;

#endif
#ifdef USE_SHEEN_COLORMAP

	varying vec2 vSheenColorMapUv;

#endif
#ifdef USE_SHEEN_ROUGHNESSMAP

	varying vec2 vSheenRoughnessMapUv;

#endif
#ifdef USE_SPECULARMAP

	varying vec2 vSpecularMapUv;

#endif
#ifdef USE_SPECULAR_COLORMAP

	varying vec2 vSpecularColorMapUv;

#endif
#ifdef USE_SPECULAR_INTENSITYMAP

	varying vec2 vSpecularIntensityMapUv;

#endif
#ifdef USE_TRANSMISSIONMAP

	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;

#endif
#ifdef USE_THICKNESSMAP

	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;

#endif
`;var hf=`
#if defined( USE_UV ) || defined( USE_ANISOTROPY )

	varying vec2 vUv;

#endif
#ifdef USE_MAP

	uniform mat3 mapTransform;
	varying vec2 vMapUv;

#endif
#ifdef USE_ALPHAMAP

	uniform mat3 alphaMapTransform;
	varying vec2 vAlphaMapUv;

#endif
#ifdef USE_LIGHTMAP

	uniform mat3 lightMapTransform;
	varying vec2 vLightMapUv;

#endif
#ifdef USE_AOMAP

	uniform mat3 aoMapTransform;
	varying vec2 vAoMapUv;

#endif
#ifdef USE_BUMPMAP

	uniform mat3 bumpMapTransform;
	varying vec2 vBumpMapUv;

#endif
#ifdef USE_NORMALMAP

	uniform mat3 normalMapTransform;
	varying vec2 vNormalMapUv;

#endif
#ifdef USE_DISPLACEMENTMAP

	uniform mat3 displacementMapTransform;
	varying vec2 vDisplacementMapUv;

#endif
#ifdef USE_EMISSIVEMAP

	uniform mat3 emissiveMapTransform;
	varying vec2 vEmissiveMapUv;

#endif
#ifdef USE_METALNESSMAP

	uniform mat3 metalnessMapTransform;
	varying vec2 vMetalnessMapUv;

#endif
#ifdef USE_ROUGHNESSMAP

	uniform mat3 roughnessMapTransform;
	varying vec2 vRoughnessMapUv;

#endif
#ifdef USE_ANISOTROPYMAP

	uniform mat3 anisotropyMapTransform;
	varying vec2 vAnisotropyMapUv;

#endif
#ifdef USE_CLEARCOATMAP

	uniform mat3 clearcoatMapTransform;
	varying vec2 vClearcoatMapUv;

#endif
#ifdef USE_CLEARCOAT_NORMALMAP

	uniform mat3 clearcoatNormalMapTransform;
	varying vec2 vClearcoatNormalMapUv;

#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP

	uniform mat3 clearcoatRoughnessMapTransform;
	varying vec2 vClearcoatRoughnessMapUv;

#endif
#ifdef USE_SHEEN_COLORMAP

	uniform mat3 sheenColorMapTransform;
	varying vec2 vSheenColorMapUv;

#endif
#ifdef USE_SHEEN_ROUGHNESSMAP

	uniform mat3 sheenRoughnessMapTransform;
	varying vec2 vSheenRoughnessMapUv;

#endif
#ifdef USE_IRIDESCENCEMAP

	uniform mat3 iridescenceMapTransform;
	varying vec2 vIridescenceMapUv;

#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP

	uniform mat3 iridescenceThicknessMapTransform;
	varying vec2 vIridescenceThicknessMapUv;

#endif
#ifdef USE_SPECULARMAP

	uniform mat3 specularMapTransform;
	varying vec2 vSpecularMapUv;

#endif
#ifdef USE_SPECULAR_COLORMAP

	uniform mat3 specularColorMapTransform;
	varying vec2 vSpecularColorMapUv;

#endif
#ifdef USE_SPECULAR_INTENSITYMAP

	uniform mat3 specularIntensityMapTransform;
	varying vec2 vSpecularIntensityMapUv;

#endif
#ifdef USE_TRANSMISSIONMAP

	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;

#endif
#ifdef USE_THICKNESSMAP

	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;

#endif
`;var ff=`
#if defined( USE_UV ) || defined( USE_ANISOTROPY )

	vUv = vec3( uv, 1 ).xy;

#endif
#ifdef USE_MAP

	vMapUv = ( mapTransform * vec3( MAP_UV, 1 ) ).xy;

#endif
#ifdef USE_ALPHAMAP

	vAlphaMapUv = ( alphaMapTransform * vec3( ALPHAMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_LIGHTMAP

	vLightMapUv = ( lightMapTransform * vec3( LIGHTMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_AOMAP

	vAoMapUv = ( aoMapTransform * vec3( AOMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_BUMPMAP

	vBumpMapUv = ( bumpMapTransform * vec3( BUMPMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_NORMALMAP

	vNormalMapUv = ( normalMapTransform * vec3( NORMALMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_DISPLACEMENTMAP

	vDisplacementMapUv = ( displacementMapTransform * vec3( DISPLACEMENTMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_EMISSIVEMAP

	vEmissiveMapUv = ( emissiveMapTransform * vec3( EMISSIVEMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_METALNESSMAP

	vMetalnessMapUv = ( metalnessMapTransform * vec3( METALNESSMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_ROUGHNESSMAP

	vRoughnessMapUv = ( roughnessMapTransform * vec3( ROUGHNESSMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_ANISOTROPYMAP

	vAnisotropyMapUv = ( anisotropyMapTransform * vec3( ANISOTROPYMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_CLEARCOATMAP

	vClearcoatMapUv = ( clearcoatMapTransform * vec3( CLEARCOATMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_CLEARCOAT_NORMALMAP

	vClearcoatNormalMapUv = ( clearcoatNormalMapTransform * vec3( CLEARCOAT_NORMALMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP

	vClearcoatRoughnessMapUv = ( clearcoatRoughnessMapTransform * vec3( CLEARCOAT_ROUGHNESSMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_IRIDESCENCEMAP

	vIridescenceMapUv = ( iridescenceMapTransform * vec3( IRIDESCENCEMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP

	vIridescenceThicknessMapUv = ( iridescenceThicknessMapTransform * vec3( IRIDESCENCE_THICKNESSMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_SHEEN_COLORMAP

	vSheenColorMapUv = ( sheenColorMapTransform * vec3( SHEEN_COLORMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_SHEEN_ROUGHNESSMAP

	vSheenRoughnessMapUv = ( sheenRoughnessMapTransform * vec3( SHEEN_ROUGHNESSMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_SPECULARMAP

	vSpecularMapUv = ( specularMapTransform * vec3( SPECULARMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_SPECULAR_COLORMAP

	vSpecularColorMapUv = ( specularColorMapTransform * vec3( SPECULAR_COLORMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_SPECULAR_INTENSITYMAP

	vSpecularIntensityMapUv = ( specularIntensityMapTransform * vec3( SPECULAR_INTENSITYMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_TRANSMISSIONMAP

	vTransmissionMapUv = ( transmissionMapTransform * vec3( TRANSMISSIONMAP_UV, 1 ) ).xy;

#endif
#ifdef USE_THICKNESSMAP

	vThicknessMapUv = ( thicknessMapTransform * vec3( THICKNESSMAP_UV, 1 ) ).xy;

#endif
`;var pf=`
#if defined( USE_ENVMAP ) || defined( DISTANCE ) || defined ( USE_SHADOWMAP ) || defined ( USE_TRANSMISSION ) || NUM_SPOT_LIGHT_COORDS > 0

	vec4 worldPosition = vec4( transformed, 1.0 );

	#ifdef USE_BATCHING

		worldPosition = batchingMatrix * worldPosition;

	#endif

	#ifdef USE_INSTANCING

		worldPosition = instanceMatrix * worldPosition;

	#endif

	worldPosition = modelMatrix * worldPosition;

#endif
`;var mf=`
varying vec2 vUv;
uniform mat3 uvTransform;

void main() {

	vUv = ( uvTransform * vec3( uv, 1 ) ).xy;

	gl_Position = vec4( position.xy, 1.0, 1.0 );

}
`,gf=`
uniform sampler2D t2D;
uniform float backgroundIntensity;

varying vec2 vUv;

void main() {

	vec4 texColor = texture2D( t2D, vUv );

	#ifdef DECODE_VIDEO_TEXTURE

		// use inline sRGB decode until browsers properly support SRGB8_APLHA8 with video textures

		texColor = vec4( mix( pow( texColor.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), texColor.rgb * 0.0773993808, vec3( lessThanEqual( texColor.rgb, vec3( 0.04045 ) ) ) ), texColor.w );

	#endif

	texColor.rgb *= backgroundIntensity;

	gl_FragColor = texColor;

	#include <tonemapping_fragment>
	#include <colorspace_fragment>

}
`;var xf=`
varying vec3 vWorldDirection;

#include <common>

void main() {

	vWorldDirection = transformDirection( position, modelMatrix );

	#include <begin_vertex>
	#include <project_vertex>

	gl_Position.z = gl_Position.w; // set z to camera.far

}
`,vf=`

#ifdef ENVMAP_TYPE_CUBE

	uniform samplerCube envMap;

#elif defined( ENVMAP_TYPE_CUBE_UV )

	uniform sampler2D envMap;

#endif

uniform float flipEnvMap;
uniform float backgroundBlurriness;
uniform float backgroundIntensity;
uniform mat3 backgroundRotation;

varying vec3 vWorldDirection;

#include <cube_uv_reflection_fragment>

void main() {

	#ifdef ENVMAP_TYPE_CUBE

		vec4 texColor = textureCube( envMap, backgroundRotation * vec3( flipEnvMap * vWorldDirection.x, vWorldDirection.yz ) );

	#elif defined( ENVMAP_TYPE_CUBE_UV )

		vec4 texColor = textureCubeUV( envMap, backgroundRotation * vWorldDirection, backgroundBlurriness );

	#else

		vec4 texColor = vec4( 0.0, 0.0, 0.0, 1.0 );

	#endif

	texColor.rgb *= backgroundIntensity;

	gl_FragColor = texColor;

	#include <tonemapping_fragment>
	#include <colorspace_fragment>

}
`;var _f=`
varying vec3 vWorldDirection;

#include <common>

void main() {

	vWorldDirection = transformDirection( position, modelMatrix );

	#include <begin_vertex>
	#include <project_vertex>

	gl_Position.z = gl_Position.w; // set z to camera.far

}
`,yf=`
uniform samplerCube tCube;
uniform float tFlip;
uniform float opacity;

varying vec3 vWorldDirection;

void main() {

	vec4 texColor = textureCube( tCube, vec3( tFlip * vWorldDirection.x, vWorldDirection.yz ) );

	gl_FragColor = texColor;
	gl_FragColor.a *= opacity;

	#include <tonemapping_fragment>
	#include <colorspace_fragment>

}
`;var Sf=`
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

// This is used for computing an equivalent of gl_FragCoord.z that is as high precision as possible.
// Some platforms compute gl_FragCoord at a lower precision which makes the manually computed value better for
// depth-based postprocessing effects. Reproduced on iPad with A10 processor / iPadOS 13.3.1.
varying vec2 vHighPrecisionZW;

void main() {

	#include <uv_vertex>

	#include <batching_vertex>
	#include <skinbase_vertex>

	#include <morphinstance_vertex>

	#ifdef USE_DISPLACEMENTMAP

		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>

	#endif

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	vHighPrecisionZW = gl_Position.zw;

}
`,bf=`
#if DEPTH_PACKING == 3200

	uniform float opacity;

#endif

#include <common>
#include <packing>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

varying vec2 vHighPrecisionZW;

void main() {

	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>

	#if DEPTH_PACKING == 3200

		diffuseColor.a = opacity;

	#endif

	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>

	#include <logdepthbuf_fragment>

	// Higher precision equivalent of gl_FragCoord.z. This assumes depthRange has been left to its default values.
	float fragCoordZ = 0.5 * vHighPrecisionZW[0] / vHighPrecisionZW[1] + 0.5;

	#if DEPTH_PACKING == 3200

		gl_FragColor = vec4( vec3( 1.0 - fragCoordZ ), opacity );

	#elif DEPTH_PACKING == 3201

		gl_FragColor = packDepthToRGBA( fragCoordZ );

	#endif

}
`;var Rf=`
#define DISTANCE

varying vec3 vWorldPosition;

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>

	#include <batching_vertex>
	#include <skinbase_vertex>

	#include <morphinstance_vertex>

	#ifdef USE_DISPLACEMENTMAP

		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>

	#endif

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <worldpos_vertex>
	#include <clipping_planes_vertex>

	vWorldPosition = worldPosition.xyz;

}
`,Mf=`
#define DISTANCE

uniform vec3 referencePosition;
uniform float nearDistance;
uniform float farDistance;
varying vec3 vWorldPosition;

#include <common>
#include <packing>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <clipping_planes_pars_fragment>

void main () {

	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>

	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>

	float dist = length( vWorldPosition - referencePosition );
	dist = ( dist - nearDistance ) / ( farDistance - nearDistance );
	dist = saturate( dist ); // clamp to [ 0, 1 ]

	gl_FragColor = packDepthToRGBA( dist );

}
`;var wf=`
varying vec3 vWorldDirection;

#include <common>

void main() {

	vWorldDirection = transformDirection( position, modelMatrix );

	#include <begin_vertex>
	#include <project_vertex>

}
`,Tf=`
uniform sampler2D tEquirect;

varying vec3 vWorldDirection;

#include <common>

void main() {

	vec3 direction = normalize( vWorldDirection );

	vec2 sampleUV = equirectUv( direction );

	gl_FragColor = texture2D( tEquirect, sampleUV );

	#include <tonemapping_fragment>
	#include <colorspace_fragment>

}
`;var Ef=`
uniform float scale;
attribute float lineDistance;

varying float vLineDistance;

#include <common>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	vLineDistance = scale * lineDistance;

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>

}
`,Af=`
uniform vec3 diffuse;
uniform float opacity;

uniform float dashSize;
uniform float totalSize;

varying float vLineDistance;

#include <common>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	if ( mod( vLineDistance, totalSize ) > dashSize ) {

		discard;

	}

	vec3 outgoingLight = vec3( 0.0 );

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>

	outgoingLight = diffuseColor.rgb; // simple shader

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>

}
`;var Cf=`
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#if defined ( USE_ENVMAP ) || defined ( USE_SKINNING )

		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinbase_vertex>
		#include <skinnormal_vertex>
		#include <defaultnormal_vertex>

	#endif

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <fog_vertex>

}
`,Pf=`
uniform vec3 diffuse;
uniform float opacity;

#ifndef FLAT_SHADED

	varying vec3 vNormal;

#endif

#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <fog_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>

	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );

	// accumulation (baked indirect lighting only)
	#ifdef USE_LIGHTMAP

		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		reflectedLight.indirectDiffuse += lightMapTexel.rgb * lightMapIntensity * RECIPROCAL_PI;

	#else

		reflectedLight.indirectDiffuse += vec3( 1.0 );

	#endif

	// modulation
	#include <aomap_fragment>

	reflectedLight.indirectDiffuse *= diffuseColor.rgb;

	vec3 outgoingLight = reflectedLight.indirectDiffuse;

	#include <envmap_fragment>

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var If=`
#define LAMBERT

varying vec3 vViewPosition;

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	vViewPosition = - mvPosition.xyz;

	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>

}
`,Df=`
#define LAMBERT

uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;

#include <common>
#include <packing>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_lambert_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>

	// accumulation
	#include <lights_lambert_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>

	// modulation
	#include <aomap_fragment>

	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;

	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var Lf=`
#define MATCAP

varying vec3 vViewPosition;

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <displacementmap_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>

#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>

	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>

	vViewPosition = - mvPosition.xyz;

}
`,Uf=`
#define MATCAP

uniform vec3 diffuse;
uniform float opacity;
uniform sampler2D matcap;

varying vec3 vViewPosition;

#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>

	vec3 viewDir = normalize( vViewPosition );
	vec3 x = normalize( vec3( viewDir.z, 0.0, - viewDir.x ) );
	vec3 y = cross( viewDir, x );
	vec2 uv = vec2( dot( x, normal ), dot( y, normal ) ) * 0.495 + 0.5; // 0.495 to remove artifacts caused by undersized matcap disks

	#ifdef USE_MATCAP

		vec4 matcapColor = texture2D( matcap, uv );

	#else

		vec4 matcapColor = vec4( vec3( mix( 0.2, 0.8, uv.y ) ), 1.0 ); // default if matcap is missing

	#endif

	vec3 outgoingLight = diffuseColor.rgb * matcapColor.rgb;

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var Ff=`
#define NORMAL

#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )

	varying vec3 vViewPosition;

#endif

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )

	vViewPosition = - mvPosition.xyz;

#endif

}
`,Of=`
#define NORMAL

uniform float opacity;

#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )

	varying vec3 vViewPosition;

#endif

#include <packing>
#include <uv_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( 0.0, 0.0, 0.0, opacity );

	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>

	gl_FragColor = vec4( packNormalToRGB( normal ), diffuseColor.a );

	#ifdef OPAQUE

		gl_FragColor.a = 1.0;

	#endif

}
`;var Nf=`
#define PHONG

varying vec3 vViewPosition;

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	vViewPosition = - mvPosition.xyz;

	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>

}
`,Bf=`
#define PHONG

uniform vec3 diffuse;
uniform vec3 emissive;
uniform vec3 specular;
uniform float shininess;
uniform float opacity;

#include <common>
#include <packing>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_phong_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>

	// accumulation
	#include <lights_phong_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>

	// modulation
	#include <aomap_fragment>

	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + reflectedLight.directSpecular + reflectedLight.indirectSpecular + totalEmissiveRadiance;

	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var kf=`
#define STANDARD

varying vec3 vViewPosition;

#ifdef USE_TRANSMISSION

	varying vec3 vWorldPosition;

#endif

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	vViewPosition = - mvPosition.xyz;

	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>

#ifdef USE_TRANSMISSION

	vWorldPosition = worldPosition.xyz;

#endif
}
`,zf=`
#define STANDARD

#ifdef PHYSICAL
	#define IOR
	#define USE_SPECULAR
#endif

uniform vec3 diffuse;
uniform vec3 emissive;
uniform float roughness;
uniform float metalness;
uniform float opacity;

#ifdef IOR
	uniform float ior;
#endif

#ifdef USE_SPECULAR
	uniform float specularIntensity;
	uniform vec3 specularColor;

	#ifdef USE_SPECULAR_COLORMAP
		uniform sampler2D specularColorMap;
	#endif

	#ifdef USE_SPECULAR_INTENSITYMAP
		uniform sampler2D specularIntensityMap;
	#endif
#endif

#ifdef USE_CLEARCOAT
	uniform float clearcoat;
	uniform float clearcoatRoughness;
#endif

#ifdef USE_DISPERSION
	uniform float dispersion;
#endif

#ifdef USE_IRIDESCENCE
	uniform float iridescence;
	uniform float iridescenceIOR;
	uniform float iridescenceThicknessMinimum;
	uniform float iridescenceThicknessMaximum;
#endif

#ifdef USE_SHEEN
	uniform vec3 sheenColor;
	uniform float sheenRoughness;

	#ifdef USE_SHEEN_COLORMAP
		uniform sampler2D sheenColorMap;
	#endif

	#ifdef USE_SHEEN_ROUGHNESSMAP
		uniform sampler2D sheenRoughnessMap;
	#endif
#endif

#ifdef USE_ANISOTROPY
	uniform vec2 anisotropyVector;

	#ifdef USE_ANISOTROPYMAP
		uniform sampler2D anisotropyMap;
	#endif
#endif

varying vec3 vViewPosition;

#include <common>
#include <packing>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <iridescence_fragment>
#include <cube_uv_reflection_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_physical_pars_fragment>
#include <transmission_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <clearcoat_pars_fragment>
#include <iridescence_pars_fragment>
#include <roughnessmap_pars_fragment>
#include <metalnessmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <roughnessmap_fragment>
	#include <metalnessmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <clearcoat_normal_fragment_begin>
	#include <clearcoat_normal_fragment_maps>
	#include <emissivemap_fragment>

	// accumulation
	#include <lights_physical_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>

	// modulation
	#include <aomap_fragment>

	vec3 totalDiffuse = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse;
	vec3 totalSpecular = reflectedLight.directSpecular + reflectedLight.indirectSpecular;

	#include <transmission_fragment>

	vec3 outgoingLight = totalDiffuse + totalSpecular + totalEmissiveRadiance;

	#ifdef USE_SHEEN

		// Sheen energy compensation approximation calculation can be found at the end of
		// https://drive.google.com/file/d/1T0D1VSyR4AllqIJTQAraEIzjlb5h4FKH/view?usp=sharing
		float sheenEnergyComp = 1.0 - 0.157 * max3( material.sheenColor );

		outgoingLight = outgoingLight * sheenEnergyComp + sheenSpecularDirect + sheenSpecularIndirect;

	#endif

	#ifdef USE_CLEARCOAT

		float dotNVcc = saturate( dot( geometryClearcoatNormal, geometryViewDir ) );

		vec3 Fcc = F_Schlick( material.clearcoatF0, material.clearcoatF90, dotNVcc );

		outgoingLight = outgoingLight * ( 1.0 - material.clearcoat * Fcc ) + ( clearcoatSpecularDirect + clearcoatSpecularIndirect ) * material.clearcoat;

	#endif

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var Vf=`
#define TOON

varying vec3 vViewPosition;

#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>

	vViewPosition = - mvPosition.xyz;

	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>

}
`,Gf=`
#define TOON

uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;

#include <common>
#include <packing>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <gradientmap_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_toon_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>

	// accumulation
	#include <lights_toon_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>

	// modulation
	#include <aomap_fragment>

	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>

}
`;var Wf=`
uniform float size;
uniform float scale;

#include <common>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

#ifdef USE_POINTS_UV

	varying vec2 vUv;
	uniform mat3 uvTransform;

#endif

void main() {

	#ifdef USE_POINTS_UV

		vUv = ( uvTransform * vec3( uv, 1 ) ).xy;

	#endif

	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>

	gl_PointSize = size;

	#ifdef USE_SIZEATTENUATION

		bool isPerspective = isPerspectiveMatrix( projectionMatrix );

		if ( isPerspective ) gl_PointSize *= ( scale / - mvPosition.z );

	#endif

	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <worldpos_vertex>
	#include <fog_vertex>

}
`,Hf=`
uniform vec3 diffuse;
uniform float opacity;

#include <common>
#include <color_pars_fragment>
#include <map_particle_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	vec3 outgoingLight = vec3( 0.0 );

	#include <logdepthbuf_fragment>
	#include <map_particle_fragment>
	#include <color_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>

	outgoingLight = diffuseColor.rgb;

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>

}
`;var jf=`
#include <common>
#include <batching_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <shadowmap_pars_vertex>

void main() {

	#include <batching_vertex>

	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>

	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>

	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>

}
`,Xf=`
uniform vec3 color;
uniform float opacity;

#include <common>
#include <packing>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <logdepthbuf_pars_fragment>
#include <shadowmap_pars_fragment>
#include <shadowmask_pars_fragment>

void main() {

	#include <logdepthbuf_fragment>

	gl_FragColor = vec4( color, opacity * ( 1.0 - getShadowMask() ) );

	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>

}
`;var qf=`
uniform float rotation;
uniform vec2 center;

#include <common>
#include <uv_pars_vertex>
#include <fog_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>

void main() {

	#include <uv_vertex>

	vec4 mvPosition = modelViewMatrix * vec4( 0.0, 0.0, 0.0, 1.0 );

	vec2 scale;
	scale.x = length( vec3( modelMatrix[ 0 ].x, modelMatrix[ 0 ].y, modelMatrix[ 0 ].z ) );
	scale.y = length( vec3( modelMatrix[ 1 ].x, modelMatrix[ 1 ].y, modelMatrix[ 1 ].z ) );

	#ifndef USE_SIZEATTENUATION

		bool isPerspective = isPerspectiveMatrix( projectionMatrix );

		if ( isPerspective ) scale *= - mvPosition.z;

	#endif

	vec2 alignedPosition = ( position.xy - ( center - vec2( 0.5 ) ) ) * scale;

	vec2 rotatedPosition;
	rotatedPosition.x = cos( rotation ) * alignedPosition.x - sin( rotation ) * alignedPosition.y;
	rotatedPosition.y = sin( rotation ) * alignedPosition.x + cos( rotation ) * alignedPosition.y;

	mvPosition.xy += rotatedPosition;

	gl_Position = projectionMatrix * mvPosition;

	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>

}
`,$f=`
uniform vec3 diffuse;
uniform float opacity;

#include <common>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>

void main() {

	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>

	vec3 outgoingLight = vec3( 0.0 );

	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>

	outgoingLight = diffuseColor.rgb;

	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>

}
`;var Ae={alphahash_fragment:mu,alphahash_pars_fragment:gu,alphamap_fragment:xu,alphamap_pars_fragment:vu,alphatest_fragment:_u,alphatest_pars_fragment:yu,aomap_fragment:Su,aomap_pars_fragment:bu,batching_pars_vertex:Ru,batching_vertex:Mu,begin_vertex:wu,beginnormal_vertex:Tu,bsdfs:Eu,iridescence_fragment:Au,bumpmap_pars_fragment:Cu,clipping_planes_fragment:Pu,clipping_planes_pars_fragment:Iu,clipping_planes_pars_vertex:Du,clipping_planes_vertex:Lu,color_fragment:Uu,color_pars_fragment:Fu,color_pars_vertex:Ou,color_vertex:Nu,common:Bu,cube_uv_reflection_fragment:ku,defaultnormal_vertex:zu,displacementmap_pars_vertex:Vu,displacementmap_vertex:Gu,emissivemap_fragment:Wu,emissivemap_pars_fragment:Hu,colorspace_fragment:ju,colorspace_pars_fragment:Xu,envmap_fragment:qu,envmap_common_pars_fragment:$u,envmap_pars_fragment:Yu,envmap_pars_vertex:Ku,envmap_physical_pars_fragment:ah,envmap_vertex:Zu,fog_vertex:Ju,fog_pars_vertex:Qu,fog_fragment:eh,fog_pars_fragment:th,gradientmap_pars_fragment:ih,lightmap_pars_fragment:rh,lights_lambert_fragment:nh,lights_lambert_pars_fragment:oh,lights_pars_begin:sh,lights_toon_fragment:ch,lights_toon_pars_fragment:lh,lights_phong_fragment:dh,lights_phong_pars_fragment:uh,lights_physical_fragment:hh,lights_physical_pars_fragment:fh,lights_fragment_begin:ph,lights_fragment_maps:mh,lights_fragment_end:gh,logdepthbuf_fragment:xh,logdepthbuf_pars_fragment:vh,logdepthbuf_pars_vertex:_h,logdepthbuf_vertex:yh,map_fragment:Sh,map_pars_fragment:bh,map_particle_fragment:Rh,map_particle_pars_fragment:Mh,metalnessmap_fragment:wh,metalnessmap_pars_fragment:Th,morphinstance_vertex:Eh,morphcolor_vertex:Ah,morphnormal_vertex:Ch,morphtarget_pars_vertex:Ph,morphtarget_vertex:Ih,normal_fragment_begin:Dh,normal_fragment_maps:Lh,normal_pars_fragment:Uh,normal_pars_vertex:Fh,normal_vertex:Oh,normalmap_pars_fragment:Nh,clearcoat_normal_fragment_begin:Bh,clearcoat_normal_fragment_maps:kh,clearcoat_pars_fragment:zh,iridescence_pars_fragment:Vh,opaque_fragment:Gh,packing:Wh,premultiplied_alpha_fragment:Hh,project_vertex:jh,dithering_fragment:Xh,dithering_pars_fragment:qh,roughnessmap_fragment:$h,roughnessmap_pars_fragment:Yh,shadowmap_pars_fragment:Kh,shadowmap_pars_vertex:Zh,shadowmap_vertex:Jh,shadowmask_pars_fragment:Qh,skinbase_vertex:ef,skinning_pars_vertex:tf,skinning_vertex:rf,skinnormal_vertex:nf,specularmap_fragment:of,specularmap_pars_fragment:sf,tonemapping_fragment:af,tonemapping_pars_fragment:cf,transmission_fragment:lf,transmission_pars_fragment:df,uv_pars_fragment:uf,uv_pars_vertex:hf,uv_vertex:ff,worldpos_vertex:pf,background_vert:mf,background_frag:gf,backgroundCube_vert:xf,backgroundCube_frag:vf,cube_vert:_f,cube_frag:yf,depth_vert:Sf,depth_frag:bf,distanceRGBA_vert:Rf,distanceRGBA_frag:Mf,equirect_vert:wf,equirect_frag:Tf,linedashed_vert:Ef,linedashed_frag:Af,meshbasic_vert:Cf,meshbasic_frag:Pf,meshlambert_vert:If,meshlambert_frag:Df,meshmatcap_vert:Lf,meshmatcap_frag:Uf,meshnormal_vert:Ff,meshnormal_frag:Of,meshphong_vert:Nf,meshphong_frag:Bf,meshphysical_vert:kf,meshphysical_frag:zf,meshtoon_vert:Vf,meshtoon_frag:Gf,points_vert:Wf,points_frag:Hf,shadow_vert:jf,shadow_frag:Xf,sprite_vert:qf,sprite_frag:$f};var ie={common:{diffuse:{value:new Me(16777215)},opacity:{value:1},map:{value:null},mapTransform:{value:new Se},alphaMap:{value:null},alphaMapTransform:{value:new Se},alphaTest:{value:0}},specularmap:{specularMap:{value:null},specularMapTransform:{value:new Se}},envmap:{envMap:{value:null},envMapRotation:{value:new Se},flipEnvMap:{value:-1},reflectivity:{value:1},ior:{value:1.5},refractionRatio:{value:.98}},aomap:{aoMap:{value:null},aoMapIntensity:{value:1},aoMapTransform:{value:new Se}},lightmap:{lightMap:{value:null},lightMapIntensity:{value:1},lightMapTransform:{value:new Se}},bumpmap:{bumpMap:{value:null},bumpMapTransform:{value:new Se},bumpScale:{value:1}},normalmap:{normalMap:{value:null},normalMapTransform:{value:new Se},normalScale:{value:new be(1,1)}},displacementmap:{displacementMap:{value:null},displacementMapTransform:{value:new Se},displacementScale:{value:1},displacementBias:{value:0}},emissivemap:{emissiveMap:{value:null},emissiveMapTransform:{value:new Se}},metalnessmap:{metalnessMap:{value:null},metalnessMapTransform:{value:new Se}},roughnessmap:{roughnessMap:{value:null},roughnessMapTransform:{value:new Se}},gradientmap:{gradientMap:{value:null}},fog:{fogDensity:{value:25e-5},fogNear:{value:1},fogFar:{value:2e3},fogColor:{value:new Me(16777215)}},lights:{ambientLightColor:{value:[]},lightProbe:{value:[]},directionalLights:{value:[],properties:{direction:{},color:{}}},directionalLightShadows:{value:[],properties:{shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},directionalShadowMap:{value:[]},directionalShadowMatrix:{value:[]},spotLights:{value:[],properties:{color:{},position:{},direction:{},distance:{},coneCos:{},penumbraCos:{},decay:{}}},spotLightShadows:{value:[],properties:{shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},spotLightMap:{value:[]},spotShadowMap:{value:[]},spotLightMatrix:{value:[]},pointLights:{value:[],properties:{color:{},position:{},decay:{},distance:{}}},pointLightShadows:{value:[],properties:{shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{},shadowCameraNear:{},shadowCameraFar:{}}},pointShadowMap:{value:[]},pointShadowMatrix:{value:[]},hemisphereLights:{value:[],properties:{direction:{},skyColor:{},groundColor:{}}},rectAreaLights:{value:[],properties:{color:{},position:{},width:{},height:{}}},ltc_1:{value:null},ltc_2:{value:null}},points:{diffuse:{value:new Me(16777215)},opacity:{value:1},size:{value:1},scale:{value:1},map:{value:null},alphaMap:{value:null},alphaMapTransform:{value:new Se},alphaTest:{value:0},uvTransform:{value:new Se}},sprite:{diffuse:{value:new Me(16777215)},opacity:{value:1},center:{value:new be(.5,.5)},rotation:{value:0},map:{value:null},mapTransform:{value:new Se},alphaMap:{value:null},alphaMapTransform:{value:new Se},alphaTest:{value:0}}};var pi={basic:{uniforms:jt([ie.common,ie.specularmap,ie.envmap,ie.aomap,ie.lightmap,ie.fog]),vertexShader:Ae.meshbasic_vert,fragmentShader:Ae.meshbasic_frag},lambert:{uniforms:jt([ie.common,ie.specularmap,ie.envmap,ie.aomap,ie.lightmap,ie.emissivemap,ie.bumpmap,ie.normalmap,ie.displacementmap,ie.fog,ie.lights,{emissive:{value:new Me(0)}}]),vertexShader:Ae.meshlambert_vert,fragmentShader:Ae.meshlambert_frag},phong:{uniforms:jt([ie.common,ie.specularmap,ie.envmap,ie.aomap,ie.lightmap,ie.emissivemap,ie.bumpmap,ie.normalmap,ie.displacementmap,ie.fog,ie.lights,{emissive:{value:new Me(0)},specular:{value:new Me(1118481)},shininess:{value:30}}]),vertexShader:Ae.meshphong_vert,fragmentShader:Ae.meshphong_frag},standard:{uniforms:jt([ie.common,ie.envmap,ie.aomap,ie.lightmap,ie.emissivemap,ie.bumpmap,ie.normalmap,ie.displacementmap,ie.roughnessmap,ie.metalnessmap,ie.fog,ie.lights,{emissive:{value:new Me(0)},roughness:{value:1},metalness:{value:0},envMapIntensity:{value:1}}]),vertexShader:Ae.meshphysical_vert,fragmentShader:Ae.meshphysical_frag},toon:{uniforms:jt([ie.common,ie.aomap,ie.lightmap,ie.emissivemap,ie.bumpmap,ie.normalmap,ie.displacementmap,ie.gradientmap,ie.fog,ie.lights,{emissive:{value:new Me(0)}}]),vertexShader:Ae.meshtoon_vert,fragmentShader:Ae.meshtoon_frag},matcap:{uniforms:jt([ie.common,ie.bumpmap,ie.normalmap,ie.displacementmap,ie.fog,{matcap:{value:null}}]),vertexShader:Ae.meshmatcap_vert,fragmentShader:Ae.meshmatcap_frag},points:{uniforms:jt([ie.points,ie.fog]),vertexShader:Ae.points_vert,fragmentShader:Ae.points_frag},dashed:{uniforms:jt([ie.common,ie.fog,{scale:{value:1},dashSize:{value:1},totalSize:{value:2}}]),vertexShader:Ae.linedashed_vert,fragmentShader:Ae.linedashed_frag},depth:{uniforms:jt([ie.common,ie.displacementmap]),vertexShader:Ae.depth_vert,fragmentShader:Ae.depth_frag},normal:{uniforms:jt([ie.common,ie.bumpmap,ie.normalmap,ie.displacementmap,{opacity:{value:1}}]),vertexShader:Ae.meshnormal_vert,fragmentShader:Ae.meshnormal_frag},sprite:{uniforms:jt([ie.sprite,ie.fog]),vertexShader:Ae.sprite_vert,fragmentShader:Ae.sprite_frag},background:{uniforms:{uvTransform:{value:new Se},t2D:{value:null},backgroundIntensity:{value:1}},vertexShader:Ae.background_vert,fragmentShader:Ae.background_frag},backgroundCube:{uniforms:{envMap:{value:null},flipEnvMap:{value:-1},backgroundBlurriness:{value:0},backgroundIntensity:{value:1},backgroundRotation:{value:new Se}},vertexShader:Ae.backgroundCube_vert,fragmentShader:Ae.backgroundCube_frag},cube:{uniforms:{tCube:{value:null},tFlip:{value:-1},opacity:{value:1}},vertexShader:Ae.cube_vert,fragmentShader:Ae.cube_frag},equirect:{uniforms:{tEquirect:{value:null}},vertexShader:Ae.equirect_vert,fragmentShader:Ae.equirect_frag},distanceRGBA:{uniforms:jt([ie.common,ie.displacementmap,{referencePosition:{value:new D},nearDistance:{value:1},farDistance:{value:1e3}}]),vertexShader:Ae.distanceRGBA_vert,fragmentShader:Ae.distanceRGBA_frag},shadow:{uniforms:jt([ie.lights,ie.fog,{color:{value:new Me(0)},opacity:{value:1}}]),vertexShader:Ae.shadow_vert,fragmentShader:Ae.shadow_frag}};pi.physical={uniforms:jt([pi.standard.uniforms,{clearcoat:{value:0},clearcoatMap:{value:null},clearcoatMapTransform:{value:new Se},clearcoatNormalMap:{value:null},clearcoatNormalMapTransform:{value:new Se},clearcoatNormalScale:{value:new be(1,1)},clearcoatRoughness:{value:0},clearcoatRoughnessMap:{value:null},clearcoatRoughnessMapTransform:{value:new Se},dispersion:{value:0},iridescence:{value:0},iridescenceMap:{value:null},iridescenceMapTransform:{value:new Se},iridescenceIOR:{value:1.3},iridescenceThicknessMinimum:{value:100},iridescenceThicknessMaximum:{value:400},iridescenceThicknessMap:{value:null},iridescenceThicknessMapTransform:{value:new Se},sheen:{value:0},sheenColor:{value:new Me(0)},sheenColorMap:{value:null},sheenColorMapTransform:{value:new Se},sheenRoughness:{value:1},sheenRoughnessMap:{value:null},sheenRoughnessMapTransform:{value:new Se},transmission:{value:0},transmissionMap:{value:null},transmissionMapTransform:{value:new Se},transmissionSamplerSize:{value:new be},transmissionSamplerMap:{value:null},thickness:{value:0},thicknessMap:{value:null},thicknessMapTransform:{value:new Se},attenuationDistance:{value:0},attenuationColor:{value:new Me(0)},specularColor:{value:new Me(1,1,1)},specularColorMap:{value:null},specularColorMapTransform:{value:new Se},specularIntensity:{value:1},specularIntensityMap:{value:null},specularIntensityMapTransform:{value:new Se},anisotropyVector:{value:new be},anisotropyMap:{value:null},anisotropyMapTransform:{value:new Se}}]),vertexShader:Ae.meshphysical_vert,fragmentShader:Ae.meshphysical_frag};var Ls={r:0,b:0,g:0},Br=new ii,Zg=new Ee;function Yf(r,e,t,i,n,o,s){let a=new Me(0),c=o===!0?0:1,l,d,u=null,h=0,p=null;function x(S){let _=S.isScene===!0?S.background:null;return _&&_.isTexture&&(_=(S.backgroundBlurriness>0?t:e).get(_)),_}function g(S){let _=!1,R=x(S);R===null?f(a,c):R&&R.isColor&&(f(R,1),_=!0);let P=r.xr.getEnvironmentBlendMode();P==="additive"?i.buffers.color.setClear(0,0,0,1,s):P==="alpha-blend"&&i.buffers.color.setClear(0,0,0,0,s),(r.autoClear||_)&&r.clear(r.autoClearColor,r.autoClearDepth,r.autoClearStencil)}function m(S,_){let R=x(_);R&&(R.isCubeTexture||R.mapping===hr)?(d===void 0&&(d=new Xe(new yr(1,1,1),new ft({name:"BackgroundCubeMaterial",uniforms:Ki(pi.backgroundCube.uniforms),vertexShader:pi.backgroundCube.vertexShader,fragmentShader:pi.backgroundCube.fragmentShader,side:ut,depthTest:!1,depthWrite:!1,fog:!1})),d.geometry.deleteAttribute("normal"),d.geometry.deleteAttribute("uv"),d.onBeforeRender=function(P,w,T){this.matrixWorld.copyPosition(T.matrixWorld)},Object.defineProperty(d.material,"envMap",{get:function(){return this.uniforms.envMap.value}}),n.update(d)),Br.copy(_.backgroundRotation),Br.x*=-1,Br.y*=-1,Br.z*=-1,R.isCubeTexture&&R.isRenderTargetTexture===!1&&(Br.y*=-1,Br.z*=-1),d.material.uniforms.envMap.value=R,d.material.uniforms.flipEnvMap.value=R.isCubeTexture&&R.isRenderTargetTexture===!1?-1:1,d.material.uniforms.backgroundBlurriness.value=_.backgroundBlurriness,d.material.uniforms.backgroundIntensity.value=_.backgroundIntensity,d.material.uniforms.backgroundRotation.value.setFromMatrix4(Zg.makeRotationFromEuler(Br)),d.material.toneMapped=Oe.getTransfer(R.colorSpace)!==$e,(u!==R||h!==R.version||p!==r.toneMapping)&&(d.material.needsUpdate=!0,u=R,h=R.version,p=r.toneMapping),d.layers.enableAll(),S.unshift(d,d.geometry,d.material,0,0,null)):R&&R.isTexture&&(l===void 0&&(l=new Xe(new ci(2,2),new ft({name:"BackgroundMaterial",uniforms:Ki(pi.background.uniforms),vertexShader:pi.background.vertexShader,fragmentShader:pi.background.fragmentShader,side:ni,depthTest:!1,depthWrite:!1,fog:!1})),l.geometry.deleteAttribute("normal"),Object.defineProperty(l.material,"map",{get:function(){return this.uniforms.t2D.value}}),n.update(l)),l.material.uniforms.t2D.value=R,l.material.uniforms.backgroundIntensity.value=_.backgroundIntensity,l.material.toneMapped=Oe.getTransfer(R.colorSpace)!==$e,R.matrixAutoUpdate===!0&&R.updateMatrix(),l.material.uniforms.uvTransform.value.copy(R.matrix),(u!==R||h!==R.version||p!==r.toneMapping)&&(l.material.needsUpdate=!0,u=R,h=R.version,p=r.toneMapping),l.layers.enableAll(),S.unshift(l,l.geometry,l.material,0,0,null))}function f(S,_){S.getRGB(Ls,ms(r)),i.buffers.color.setClear(Ls.r,Ls.g,Ls.b,_,s)}return{getClearColor:function(){return a},setClearColor:function(S,_=1){a.set(S),c=_,f(a,c)},getClearAlpha:function(){return c},setClearAlpha:function(S){c=S,f(a,c)},render:g,addToRenderList:m}}function Kf(r,e){let t=r.getParameter(r.MAX_VERTEX_ATTRIBS),i={},n=h(null),o=n,s=!1;function a(v,I,F,E,N){let W=!1,X=u(E,F,I);o!==X&&(o=X,l(o.object)),W=p(v,E,F,N),W&&x(v,E,F,N),N!==null&&e.update(N,r.ELEMENT_ARRAY_BUFFER),(W||s)&&(s=!1,R(v,I,F,E),N!==null&&r.bindBuffer(r.ELEMENT_ARRAY_BUFFER,e.get(N).buffer))}function c(){return r.createVertexArray()}function l(v){return r.bindVertexArray(v)}function d(v){return r.deleteVertexArray(v)}function u(v,I,F){let E=F.wireframe===!0,N=i[v.id];N===void 0&&(N={},i[v.id]=N);let W=N[I.id];W===void 0&&(W={},N[I.id]=W);let X=W[E];return X===void 0&&(X=h(c()),W[E]=X),X}function h(v){let I=[],F=[],E=[];for(let N=0;N<t;N++)I[N]=0,F[N]=0,E[N]=0;return{geometry:null,program:null,wireframe:!1,newAttributes:I,enabledAttributes:F,attributeDivisors:E,object:v,attributes:{},index:null}}function p(v,I,F,E){let N=o.attributes,W=I.attributes,X=0,te=F.getAttributes();for(let V in te)if(te[V].location>=0){let Q=N[V],me=W[V];if(me===void 0&&(V==="instanceMatrix"&&v.instanceMatrix&&(me=v.instanceMatrix),V==="instanceColor"&&v.instanceColor&&(me=v.instanceColor)),Q===void 0||Q.attribute!==me||me&&Q.data!==me.data)return!0;X++}return o.attributesNum!==X||o.index!==E}function x(v,I,F,E){let N={},W=I.attributes,X=0,te=F.getAttributes();for(let V in te)if(te[V].location>=0){let Q=W[V];Q===void 0&&(V==="instanceMatrix"&&v.instanceMatrix&&(Q=v.instanceMatrix),V==="instanceColor"&&v.instanceColor&&(Q=v.instanceColor));let me={};me.attribute=Q,Q&&Q.data&&(me.data=Q.data),N[V]=me,X++}o.attributes=N,o.attributesNum=X,o.index=E}function g(){let v=o.newAttributes;for(let I=0,F=v.length;I<F;I++)v[I]=0}function m(v){f(v,0)}function f(v,I){let F=o.newAttributes,E=o.enabledAttributes,N=o.attributeDivisors;F[v]=1,E[v]===0&&(r.enableVertexAttribArray(v),E[v]=1),N[v]!==I&&(r.vertexAttribDivisor(v,I),N[v]=I)}function S(){let v=o.newAttributes,I=o.enabledAttributes;for(let F=0,E=I.length;F<E;F++)I[F]!==v[F]&&(r.disableVertexAttribArray(F),I[F]=0)}function _(v,I,F,E,N,W,X){X===!0?r.vertexAttribIPointer(v,I,F,N,W):r.vertexAttribPointer(v,I,F,E,N,W)}function R(v,I,F,E){g();let N=E.attributes,W=F.getAttributes(),X=I.defaultAttributeValues;for(let te in W){let V=W[te];if(V.location>=0){let J=N[te];if(J===void 0&&(te==="instanceMatrix"&&v.instanceMatrix&&(J=v.instanceMatrix),te==="instanceColor"&&v.instanceColor&&(J=v.instanceColor)),J!==void 0){let Q=J.normalized,me=J.itemSize,Ne=e.get(J);if(Ne===void 0)continue;let tt=Ne.buffer,H=Ne.type,ee=Ne.bytesPerElement,he=H===r.INT||H===r.UNSIGNED_INT||J.gpuType===Yo;if(J.isInterleavedBufferAttribute){let ne=J.data,Be=ne.stride,ke=J.offset;if(ne.isInstancedInterleavedBuffer){for(let O=0;O<V.locationSize;O++)f(V.location+O,ne.meshPerAttribute);v.isInstancedMesh!==!0&&E._maxInstanceCount===void 0&&(E._maxInstanceCount=ne.meshPerAttribute*ne.count)}else for(let O=0;O<V.locationSize;O++)m(V.location+O);r.bindBuffer(r.ARRAY_BUFFER,tt);for(let O=0;O<V.locationSize;O++)_(V.location+O,me/V.locationSize,H,Q,Be*ee,(ke+me/V.locationSize*O)*ee,he)}else{if(J.isInstancedBufferAttribute){for(let ne=0;ne<V.locationSize;ne++)f(V.location+ne,J.meshPerAttribute);v.isInstancedMesh!==!0&&E._maxInstanceCount===void 0&&(E._maxInstanceCount=J.meshPerAttribute*J.count)}else for(let ne=0;ne<V.locationSize;ne++)m(V.location+ne);r.bindBuffer(r.ARRAY_BUFFER,tt);for(let ne=0;ne<V.locationSize;ne++)_(V.location+ne,me/V.locationSize,H,Q,me*ee,me/V.locationSize*ne*ee,he)}}else if(X!==void 0){let Q=X[te];if(Q!==void 0)switch(Q.length){case 2:r.vertexAttrib2fv(V.location,Q);break;case 3:r.vertexAttrib3fv(V.location,Q);break;case 4:r.vertexAttrib4fv(V.location,Q);break;default:r.vertexAttrib1fv(V.location,Q)}}}}S()}function P(){C();for(let v in i){let I=i[v];for(let F in I){let E=I[F];for(let N in E)d(E[N].object),delete E[N];delete I[F]}delete i[v]}}function w(v){if(i[v.id]===void 0)return;let I=i[v.id];for(let F in I){let E=I[F];for(let N in E)d(E[N].object),delete E[N];delete I[F]}delete i[v.id]}function T(v){for(let I in i){let F=i[I];if(F[v.id]===void 0)continue;let E=F[v.id];for(let N in E)d(E[N].object),delete E[N];delete F[v.id]}}function C(){y(),s=!0,o!==n&&(o=n,l(o.object))}function y(){n.geometry=null,n.program=null,n.wireframe=!1}return{setup:a,reset:C,resetDefaultState:y,dispose:P,releaseStatesOfGeometry:w,releaseStatesOfProgram:T,initAttributes:g,enableAttribute:m,disableUnusedAttributes:S}}function Zf(r,e,t){let i;function n(l){i=l}function o(l,d){r.drawArrays(i,l,d),t.update(d,i,1)}function s(l,d,u){u!==0&&(r.drawArraysInstanced(i,l,d,u),t.update(d,i,u))}function a(l,d,u){if(u===0)return;let h=e.get("WEBGL_multi_draw");if(h===null)for(let p=0;p<u;p++)this.render(l[p],d[p]);else{h.multiDrawArraysWEBGL(i,l,0,d,0,u);let p=0;for(let x=0;x<u;x++)p+=d[x];t.update(p,i,1)}}function c(l,d,u,h){if(u===0)return;let p=e.get("WEBGL_multi_draw");if(p===null)for(let x=0;x<l.length;x++)s(l[x],d[x],h[x]);else{p.multiDrawArraysInstancedWEBGL(i,l,0,d,0,h,0,u);let x=0;for(let g=0;g<u;g++)x+=d[g];for(let g=0;g<h.length;g++)t.update(x,i,h[g])}}this.setMode=n,this.render=o,this.renderInstances=s,this.renderMultiDraw=a,this.renderMultiDrawInstances=c}function Jf(r,e,t,i){let n;function o(){if(n!==void 0)return n;if(e.has("EXT_texture_filter_anisotropic")===!0){let w=e.get("EXT_texture_filter_anisotropic");n=r.getParameter(w.MAX_TEXTURE_MAX_ANISOTROPY_EXT)}else n=0;return n}function s(w){return!(w!==Mt&&i.convert(w)!==r.getParameter(r.IMPLEMENTATION_COLOR_READ_FORMAT))}function a(w){let T=w===fr&&(e.has("EXT_color_buffer_half_float")||e.has("EXT_color_buffer_float"));return!(w!==Ut&&i.convert(w)!==r.getParameter(r.IMPLEMENTATION_COLOR_READ_TYPE)&&w!==oi&&!T)}function c(w){if(w==="highp"){if(r.getShaderPrecisionFormat(r.VERTEX_SHADER,r.HIGH_FLOAT).precision>0&&r.getShaderPrecisionFormat(r.FRAGMENT_SHADER,r.HIGH_FLOAT).precision>0)return"highp";w="mediump"}return w==="mediump"&&r.getShaderPrecisionFormat(r.VERTEX_SHADER,r.MEDIUM_FLOAT).precision>0&&r.getShaderPrecisionFormat(r.FRAGMENT_SHADER,r.MEDIUM_FLOAT).precision>0?"mediump":"lowp"}let l=t.precision!==void 0?t.precision:"highp",d=c(l);d!==l&&(console.warn("THREE.WebGLRenderer:",l,"not supported, using",d,"instead."),l=d);let u=t.logarithmicDepthBuffer===!0,h=r.getParameter(r.MAX_TEXTURE_IMAGE_UNITS),p=r.getParameter(r.MAX_VERTEX_TEXTURE_IMAGE_UNITS),x=r.getParameter(r.MAX_TEXTURE_SIZE),g=r.getParameter(r.MAX_CUBE_MAP_TEXTURE_SIZE),m=r.getParameter(r.MAX_VERTEX_ATTRIBS),f=r.getParameter(r.MAX_VERTEX_UNIFORM_VECTORS),S=r.getParameter(r.MAX_VARYING_VECTORS),_=r.getParameter(r.MAX_FRAGMENT_UNIFORM_VECTORS),R=p>0,P=r.getParameter(r.MAX_SAMPLES);return{isWebGL2:!0,getMaxAnisotropy:o,getMaxPrecision:c,textureFormatReadable:s,textureTypeReadable:a,precision:l,logarithmicDepthBuffer:u,maxTextures:h,maxVertexTextures:p,maxTextureSize:x,maxCubemapSize:g,maxAttributes:m,maxVertexUniforms:f,maxVaryings:S,maxFragmentUniforms:_,vertexTextures:R,maxSamples:P}}function Qf(r){let e=this,t=null,i=0,n=!1,o=!1,s=new Mi,a=new Se,c={value:null,needsUpdate:!1};this.uniform=c,this.numPlanes=0,this.numIntersection=0,this.init=function(u,h){let p=u.length!==0||h||i!==0||n;return n=h,i=u.length,p},this.beginShadows=function(){o=!0,d(null)},this.endShadows=function(){o=!1},this.setGlobalState=function(u,h){t=d(u,h,0)},this.setState=function(u,h,p){let x=u.clippingPlanes,g=u.clipIntersection,m=u.clipShadows,f=r.get(u);if(!n||x===null||x.length===0||o&&!m)o?d(null):l();else{let S=o?0:i,_=S*4,R=f.clippingState||null;c.value=R,R=d(x,h,_,p);for(let P=0;P!==_;++P)R[P]=t[P];f.clippingState=R,this.numIntersection=g?this.numPlanes:0,this.numPlanes+=S}};function l(){c.value!==t&&(c.value=t,c.needsUpdate=i>0),e.numPlanes=i,e.numIntersection=0}function d(u,h,p,x){let g=u!==null?u.length:0,m=null;if(g!==0){if(m=c.value,x!==!0||m===null){let f=p+g*4,S=h.matrixWorldInverse;a.getNormalMatrix(S),(m===null||m.length<f)&&(m=new Float32Array(f));for(let _=0,R=p;_!==g;++_,R+=4)s.copy(u[_]).applyMatrix4(S,a),s.normal.toArray(m,R),m[R+3]=s.constant}c.value=m,c.needsUpdate=!0}return e.numPlanes=g,e.numIntersection=0,m}}var _n=-90,yn=1,Us=class extends vt{constructor(e,t,i){super(),this.type="CubeCamera",this.renderTarget=i,this.coordinateSystem=null,this.activeMipmapLevel=0;let n=new Bt(_n,yn,e,t);n.layers=this.layers,this.add(n);let o=new Bt(_n,yn,e,t);o.layers=this.layers,this.add(o);let s=new Bt(_n,yn,e,t);s.layers=this.layers,this.add(s);let a=new Bt(_n,yn,e,t);a.layers=this.layers,this.add(a);let c=new Bt(_n,yn,e,t);c.layers=this.layers,this.add(c);let l=new Bt(_n,yn,e,t);l.layers=this.layers,this.add(l)}updateCoordinateSystem(){let e=this.coordinateSystem,t=this.children.concat(),[i,n,o,s,a,c]=t;for(let l of t)this.remove(l);if(e===ti)i.up.set(0,1,0),i.lookAt(1,0,0),n.up.set(0,1,0),n.lookAt(-1,0,0),o.up.set(0,0,-1),o.lookAt(0,1,0),s.up.set(0,0,1),s.lookAt(0,-1,0),a.up.set(0,1,0),a.lookAt(0,0,1),c.up.set(0,1,0),c.lookAt(0,0,-1);else if(e===Lr)i.up.set(0,-1,0),i.lookAt(-1,0,0),n.up.set(0,-1,0),n.lookAt(1,0,0),o.up.set(0,0,1),o.lookAt(0,1,0),s.up.set(0,0,-1),s.lookAt(0,-1,0),a.up.set(0,-1,0),a.lookAt(0,0,1),c.up.set(0,-1,0),c.lookAt(0,0,-1);else throw new Error("THREE.CubeCamera.updateCoordinateSystem(): Invalid coordinate system: "+e);for(let l of t)this.add(l),l.updateMatrixWorld()}update(e,t){this.parent===null&&this.updateMatrixWorld();let{renderTarget:i,activeMipmapLevel:n}=this;this.coordinateSystem!==e.coordinateSystem&&(this.coordinateSystem=e.coordinateSystem,this.updateCoordinateSystem());let[o,s,a,c,l,d]=this.children,u=e.getRenderTarget(),h=e.getActiveCubeFace(),p=e.getActiveMipmapLevel(),x=e.xr.enabled;e.xr.enabled=!1;let g=i.texture.generateMipmaps;i.texture.generateMipmaps=!1,e.setRenderTarget(i,0,n),e.render(t,o),e.setRenderTarget(i,1,n),e.render(t,s),e.setRenderTarget(i,2,n),e.render(t,a),e.setRenderTarget(i,3,n),e.render(t,c),e.setRenderTarget(i,4,n),e.render(t,l),i.texture.generateMipmaps=g,e.setRenderTarget(i,5,n),e.render(t,d),e.setRenderTarget(u,h,p),e.xr.enabled=x,i.texture.needsPMREMUpdate=!0}};var Sn=class extends St{constructor(e,t,i,n,o,s,a,c,l,d){e=e!==void 0?e:[],t=t!==void 0?t:vi,super(e,t,i,n,o,s,a,c,l,d),this.isCubeTexture=!0,this.flipY=!1}get images(){return this.image}set images(e){this.image=e}};var Fs=class extends Ot{constructor(e=1,t={}){super(e,e,t),this.isWebGLCubeRenderTarget=!0;let i={width:e,height:e,depth:1},n=[i,i,i,i,i,i];this.texture=new Sn(n,t.mapping,t.wrapS,t.wrapT,t.magFilter,t.minFilter,t.format,t.type,t.anisotropy,t.colorSpace),this.texture.isRenderTargetTexture=!0,this.texture.generateMipmaps=t.generateMipmaps!==void 0?t.generateMipmaps:!1,this.texture.minFilter=t.minFilter!==void 0?t.minFilter:ht}fromEquirectangularTexture(e,t){this.texture.type=t.type,this.texture.colorSpace=t.colorSpace,this.texture.generateMipmaps=t.generateMipmaps,this.texture.minFilter=t.minFilter,this.texture.magFilter=t.magFilter;let i={uniforms:{tEquirect:{value:null}},vertexShader:`

				varying vec3 vWorldDirection;

				vec3 transformDirection( in vec3 dir, in mat4 matrix ) {

					return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );

				}

				void main() {

					vWorldDirection = transformDirection( position, modelMatrix );

					#include <begin_vertex>
					#include <project_vertex>

				}
			`,fragmentShader:`

				uniform sampler2D tEquirect;

				varying vec3 vWorldDirection;

				#include <common>

				void main() {

					vec3 direction = normalize( vWorldDirection );

					vec2 sampleUV = equirectUv( direction );

					gl_FragColor = texture2D( tEquirect, sampleUV );

				}
			`},n=new yr(5,5,5),o=new ft({name:"CubemapFromEquirect",uniforms:Ki(i.uniforms),vertexShader:i.vertexShader,fragmentShader:i.fragmentShader,side:ut,blending:di});o.uniforms.tEquirect.value=t;let s=new Xe(n,o),a=t.minFilter;return t.minFilter===_i&&(t.minFilter=ht),new Us(1,10,this).update(e,s),t.minFilter=a,s.geometry.dispose(),s.material.dispose(),this}clear(e,t,i,n){let o=e.getRenderTarget();for(let s=0;s<6;s++)e.setRenderTarget(this,s),e.clear(t,i,n);e.setRenderTarget(o)}};function ep(r){let e=new WeakMap;function t(s,a){return a===to?s.mapping=vi:a===io&&(s.mapping=Ui),s}function i(s){if(s&&s.isTexture){let a=s.mapping;if(a===to||a===io)if(e.has(s)){let c=e.get(s).texture;return t(c,s.mapping)}else{let c=s.image;if(c&&c.height>0){let l=new Fs(c.height);return l.fromEquirectangularTexture(r,s),e.set(s,l),s.addEventListener("dispose",n),t(l.texture,s.mapping)}else return null}}return s}function n(s){let a=s.target;a.removeEventListener("dispose",n);let c=e.get(a);c!==void 0&&(e.delete(a),c.dispose())}function o(){e=new WeakMap}return{get:i,dispose:o}}var Rn=4,tp=[.125,.215,.35,.446,.526,.582],zr=20,mc=new un,ip=new Me,gc=null,xc=0,vc=0,_c=!1,kr=(1+Math.sqrt(5))/2,bn=1/kr,rp=[new D(-kr,bn,0),new D(kr,bn,0),new D(-bn,0,kr),new D(bn,0,kr),new D(0,kr,-bn),new D(0,kr,bn),new D(-1,1,-1),new D(1,1,-1),new D(-1,1,1),new D(1,1,1)],fo=class{constructor(e){this._renderer=e,this._pingPongRenderTarget=null,this._lodMax=0,this._cubeSize=0,this._lodPlanes=[],this._sizeLods=[],this._sigmas=[],this._blurMaterial=null,this._cubemapMaterial=null,this._equirectMaterial=null,this._compileMaterial(this._blurMaterial)}fromScene(e,t=0,i=.1,n=100){gc=this._renderer.getRenderTarget(),xc=this._renderer.getActiveCubeFace(),vc=this._renderer.getActiveMipmapLevel(),_c=this._renderer.xr.enabled,this._renderer.xr.enabled=!1,this._setSize(256);let o=this._allocateTargets();return o.depthBuffer=!0,this._sceneToCubeUV(e,i,n,o),t>0&&this._blur(o,0,0,t),this._applyPMREM(o),this._cleanup(o),o}fromEquirectangular(e,t=null){return this._fromTexture(e,t)}fromCubemap(e,t=null){return this._fromTexture(e,t)}compileCubemapShader(){this._cubemapMaterial===null&&(this._cubemapMaterial=sp(),this._compileMaterial(this._cubemapMaterial))}compileEquirectangularShader(){this._equirectMaterial===null&&(this._equirectMaterial=op(),this._compileMaterial(this._equirectMaterial))}dispose(){this._dispose(),this._cubemapMaterial!==null&&this._cubemapMaterial.dispose(),this._equirectMaterial!==null&&this._equirectMaterial.dispose()}_setSize(e){this._lodMax=Math.floor(Math.log2(e)),this._cubeSize=Math.pow(2,this._lodMax)}_dispose(){this._blurMaterial!==null&&this._blurMaterial.dispose(),this._pingPongRenderTarget!==null&&this._pingPongRenderTarget.dispose();for(let e=0;e<this._lodPlanes.length;e++)this._lodPlanes[e].dispose()}_cleanup(e){this._renderer.setRenderTarget(gc,xc,vc),this._renderer.xr.enabled=_c,e.scissorTest=!1,Os(e,0,0,e.width,e.height)}_fromTexture(e,t){e.mapping===vi||e.mapping===Ui?this._setSize(e.image.length===0?16:e.image[0].width||e.image[0].image.width):this._setSize(e.image.width/4),gc=this._renderer.getRenderTarget(),xc=this._renderer.getActiveCubeFace(),vc=this._renderer.getActiveMipmapLevel(),_c=this._renderer.xr.enabled,this._renderer.xr.enabled=!1;let i=t||this._allocateTargets();return this._textureToCubeUV(e,i),this._applyPMREM(i),this._cleanup(i),i}_allocateTargets(){let e=3*Math.max(this._cubeSize,112),t=4*this._cubeSize,i={magFilter:ht,minFilter:ht,generateMipmaps:!1,type:fr,format:Mt,colorSpace:ei,depthBuffer:!1},n=np(e,t,i);if(this._pingPongRenderTarget===null||this._pingPongRenderTarget.width!==e||this._pingPongRenderTarget.height!==t){this._pingPongRenderTarget!==null&&this._dispose(),this._pingPongRenderTarget=np(e,t,i);let{_lodMax:o}=this;({sizeLods:this._sizeLods,lodPlanes:this._lodPlanes,sigmas:this._sigmas}=Jg(o)),this._blurMaterial=Qg(o,e,t)}return n}_compileMaterial(e){let t=new Xe(this._lodPlanes[0],e);this._renderer.compile(t,mc)}_sceneToCubeUV(e,t,i,n){let a=new Bt(90,1,t,i),c=[1,-1,1,1,1,1],l=[1,1,1,-1,-1,-1],d=this._renderer,u=d.autoClear,h=d.toneMapping;d.getClearColor(ip),d.toneMapping=ui,d.autoClear=!1;let p=new bi({name:"PMREM.Background",side:ut,depthWrite:!1,depthTest:!1}),x=new Xe(new yr,p),g=!1,m=e.background;m?m.isColor&&(p.color.copy(m),e.background=null,g=!0):(p.color.copy(ip),g=!0);for(let f=0;f<6;f++){let S=f%3;S===0?(a.up.set(0,c[f],0),a.lookAt(l[f],0,0)):S===1?(a.up.set(0,0,c[f]),a.lookAt(0,l[f],0)):(a.up.set(0,c[f],0),a.lookAt(0,0,l[f]));let _=this._cubeSize;Os(n,S*_,f>2?_:0,_,_),d.setRenderTarget(n),g&&d.render(x,a),d.render(e,a)}x.geometry.dispose(),x.material.dispose(),d.toneMapping=h,d.autoClear=u,e.background=m}_textureToCubeUV(e,t){let i=this._renderer,n=e.mapping===vi||e.mapping===Ui;n?(this._cubemapMaterial===null&&(this._cubemapMaterial=sp()),this._cubemapMaterial.uniforms.flipEnvMap.value=e.isRenderTargetTexture===!1?-1:1):this._equirectMaterial===null&&(this._equirectMaterial=op());let o=n?this._cubemapMaterial:this._equirectMaterial,s=new Xe(this._lodPlanes[0],o),a=o.uniforms;a.envMap.value=e;let c=this._cubeSize;Os(t,0,0,3*c,2*c),i.setRenderTarget(t),i.render(s,mc)}_applyPMREM(e){let t=this._renderer,i=t.autoClear;t.autoClear=!1;let n=this._lodPlanes.length;for(let o=1;o<n;o++){let s=Math.sqrt(this._sigmas[o]*this._sigmas[o]-this._sigmas[o-1]*this._sigmas[o-1]),a=rp[(n-o-1)%rp.length];this._blur(e,o-1,o,s,a)}t.autoClear=i}_blur(e,t,i,n,o){let s=this._pingPongRenderTarget;this._halfBlur(e,s,t,i,n,"latitudinal",o),this._halfBlur(s,e,i,i,n,"longitudinal",o)}_halfBlur(e,t,i,n,o,s,a){let c=this._renderer,l=this._blurMaterial;s!=="latitudinal"&&s!=="longitudinal"&&console.error("blur direction must be either latitudinal or longitudinal!");let d=3,u=new Xe(this._lodPlanes[n],l),h=l.uniforms,p=this._sizeLods[i]-1,x=isFinite(o)?Math.PI/(2*p):2*Math.PI/(2*zr-1),g=o/x,m=isFinite(o)?1+Math.floor(d*g):zr;m>zr&&console.warn(`sigmaRadians, ${o}, is too large and will clip, as it requested ${m} samples when the maximum is set to ${zr}`);let f=[],S=0;for(let T=0;T<zr;++T){let C=T/g,y=Math.exp(-C*C/2);f.push(y),T===0?S+=y:T<m&&(S+=2*y)}for(let T=0;T<f.length;T++)f[T]=f[T]/S;h.envMap.value=e.texture,h.samples.value=m,h.weights.value=f,h.latitudinal.value=s==="latitudinal",a&&(h.poleAxis.value=a);let{_lodMax:_}=this;h.dTheta.value=x,h.mipInt.value=_-i;let R=this._sizeLods[n],P=3*R*(n>_-Rn?n-_+Rn:0),w=4*(this._cubeSize-R);Os(t,P,w,3*R,2*R),c.setRenderTarget(t),c.render(u,mc)}};function Jg(r){let e=[],t=[],i=[],n=r,o=r-Rn+1+tp.length;for(let s=0;s<o;s++){let a=Math.pow(2,n);t.push(a);let c=1/a;s>r-Rn?c=tp[s-r+Rn-1]:s===0&&(c=0),i.push(c);let l=1/(a-2),d=-l,u=1+l,h=[d,d,u,d,u,u,d,d,u,u,d,u],p=6,x=6,g=3,m=2,f=1,S=new Float32Array(g*x*p),_=new Float32Array(m*x*p),R=new Float32Array(f*x*p);for(let w=0;w<p;w++){let T=w%3*2/3-1,C=w>2?0:-1,y=[T,C,0,T+2/3,C,0,T+2/3,C+1,0,T,C,0,T+2/3,C+1,0,T,C+1,0];S.set(y,g*x*w),_.set(h,m*x*w);let v=[w,w,w,w,w,w];R.set(v,f*x*w)}let P=new Tt;P.setAttribute("position",new Kt(S,g)),P.setAttribute("uv",new Kt(_,m)),P.setAttribute("faceIndex",new Kt(R,f)),e.push(P),n>Rn&&n--}return{lodPlanes:e,sizeLods:t,sigmas:i}}function np(r,e,t){let i=new Ot(r,e,t);return i.texture.mapping=hr,i.texture.name="PMREM.cubeUv",i.scissorTest=!0,i}function Os(r,e,t,i,n){r.viewport.set(e,t,i,n),r.scissor.set(e,t,i,n)}function Qg(r,e,t){let i=new Float32Array(zr),n=new D(0,1,0);return new ft({name:"SphericalGaussianBlur",defines:{n:zr,CUBEUV_TEXEL_WIDTH:1/e,CUBEUV_TEXEL_HEIGHT:1/t,CUBEUV_MAX_MIP:`${r}.0`},uniforms:{envMap:{value:null},samples:{value:1},weights:{value:i},latitudinal:{value:!1},dTheta:{value:0},mipInt:{value:0},poleAxis:{value:n}},vertexShader:yc(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			varying vec3 vOutputDirection;

			uniform sampler2D envMap;
			uniform int samples;
			uniform float weights[ n ];
			uniform bool latitudinal;
			uniform float dTheta;
			uniform float mipInt;
			uniform vec3 poleAxis;

			#define ENVMAP_TYPE_CUBE_UV
			#include <cube_uv_reflection_fragment>

			vec3 getSample( float theta, vec3 axis ) {

				float cosTheta = cos( theta );
				// Rodrigues' axis-angle rotation
				vec3 sampleDirection = vOutputDirection * cosTheta
					+ cross( axis, vOutputDirection ) * sin( theta )
					+ axis * dot( axis, vOutputDirection ) * ( 1.0 - cosTheta );

				return bilinearCubeUV( envMap, sampleDirection, mipInt );

			}

			void main() {

				vec3 axis = latitudinal ? poleAxis : cross( poleAxis, vOutputDirection );

				if ( all( equal( axis, vec3( 0.0 ) ) ) ) {

					axis = vec3( vOutputDirection.z, 0.0, - vOutputDirection.x );

				}

				axis = normalize( axis );

				gl_FragColor = vec4( 0.0, 0.0, 0.0, 1.0 );
				gl_FragColor.rgb += weights[ 0 ] * getSample( 0.0, axis );

				for ( int i = 1; i < n; i++ ) {

					if ( i >= samples ) {

						break;

					}

					float theta = dTheta * float( i );
					gl_FragColor.rgb += weights[ i ] * getSample( -1.0 * theta, axis );
					gl_FragColor.rgb += weights[ i ] * getSample( theta, axis );

				}

			}
		`,blending:di,depthTest:!1,depthWrite:!1})}function op(){return new ft({name:"EquirectangularToCubeUV",uniforms:{envMap:{value:null}},vertexShader:yc(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			varying vec3 vOutputDirection;

			uniform sampler2D envMap;

			#include <common>

			void main() {

				vec3 outputDirection = normalize( vOutputDirection );
				vec2 uv = equirectUv( outputDirection );

				gl_FragColor = vec4( texture2D ( envMap, uv ).rgb, 1.0 );

			}
		`,blending:di,depthTest:!1,depthWrite:!1})}function sp(){return new ft({name:"CubemapToCubeUV",uniforms:{envMap:{value:null},flipEnvMap:{value:-1}},vertexShader:yc(),fragmentShader:`

			precision mediump float;
			precision mediump int;

			uniform float flipEnvMap;

			varying vec3 vOutputDirection;

			uniform samplerCube envMap;

			void main() {

				gl_FragColor = textureCube( envMap, vec3( flipEnvMap * vOutputDirection.x, vOutputDirection.yz ) );

			}
		`,blending:di,depthTest:!1,depthWrite:!1})}function yc(){return`

		precision mediump float;
		precision mediump int;

		attribute float faceIndex;

		varying vec3 vOutputDirection;

		// RH coordinate system; PMREM face-indexing convention
		vec3 getDirection( vec2 uv, float face ) {

			uv = 2.0 * uv - 1.0;

			vec3 direction = vec3( uv, 1.0 );

			if ( face == 0.0 ) {

				direction = direction.zyx; // ( 1, v, u ) pos x

			} else if ( face == 1.0 ) {

				direction = direction.xzy;
				direction.xz *= -1.0; // ( -u, 1, -v ) pos y

			} else if ( face == 2.0 ) {

				direction.x *= -1.0; // ( -u, v, 1 ) pos z

			} else if ( face == 3.0 ) {

				direction = direction.zyx;
				direction.xz *= -1.0; // ( -1, v, -u ) neg x

			} else if ( face == 4.0 ) {

				direction = direction.xzy;
				direction.xy *= -1.0; // ( -u, -1, v ) neg y

			} else if ( face == 5.0 ) {

				direction.z *= -1.0; // ( u, v, -1 ) neg z

			}

			return direction;

		}

		void main() {

			vOutputDirection = getDirection( uv, faceIndex );
			gl_Position = vec4( position, 1.0 );

		}
	`}function ap(r){let e=new WeakMap,t=null;function i(a){if(a&&a.isTexture){let c=a.mapping,l=c===to||c===io,d=c===vi||c===Ui;if(l||d){let u=e.get(a),h=u!==void 0?u.texture.pmremVersion:0;if(a.isRenderTargetTexture&&a.pmremVersion!==h)return t===null&&(t=new fo(r)),u=l?t.fromEquirectangular(a,u):t.fromCubemap(a,u),u.texture.pmremVersion=a.pmremVersion,e.set(a,u),u.texture;if(u!==void 0)return u.texture;{let p=a.image;return l&&p&&p.height>0||d&&p&&n(p)?(t===null&&(t=new fo(r)),u=l?t.fromEquirectangular(a):t.fromCubemap(a),u.texture.pmremVersion=a.pmremVersion,e.set(a,u),a.addEventListener("dispose",o),u.texture):null}}}return a}function n(a){let c=0,l=6;for(let d=0;d<l;d++)a[d]!==void 0&&c++;return c===l}function o(a){let c=a.target;c.removeEventListener("dispose",o);let l=e.get(c);l!==void 0&&(e.delete(c),l.dispose())}function s(){e=new WeakMap,t!==null&&(t.dispose(),t=null)}return{get:i,dispose:s}}function cp(r){let e={};function t(i){if(e[i]!==void 0)return e[i];let n;switch(i){case"WEBGL_depth_texture":n=r.getExtension("WEBGL_depth_texture")||r.getExtension("MOZ_WEBGL_depth_texture")||r.getExtension("WEBKIT_WEBGL_depth_texture");break;case"EXT_texture_filter_anisotropic":n=r.getExtension("EXT_texture_filter_anisotropic")||r.getExtension("MOZ_EXT_texture_filter_anisotropic")||r.getExtension("WEBKIT_EXT_texture_filter_anisotropic");break;case"WEBGL_compressed_texture_s3tc":n=r.getExtension("WEBGL_compressed_texture_s3tc")||r.getExtension("MOZ_WEBGL_compressed_texture_s3tc")||r.getExtension("WEBKIT_WEBGL_compressed_texture_s3tc");break;case"WEBGL_compressed_texture_pvrtc":n=r.getExtension("WEBGL_compressed_texture_pvrtc")||r.getExtension("WEBKIT_WEBGL_compressed_texture_pvrtc");break;default:n=r.getExtension(i)}return e[i]=n,n}return{has:function(i){return t(i)!==null},init:function(){t("EXT_color_buffer_float"),t("WEBGL_clip_cull_distance"),t("OES_texture_float_linear"),t("EXT_color_buffer_half_float"),t("WEBGL_multisampled_render_to_texture"),t("WEBGL_render_shared_exponent")},get:function(i){let n=t(i);return n===null&&console.warn("THREE.WebGLRenderer: "+i+" extension not supported."),n}}}function lp(r,e,t,i){let n={},o=new WeakMap;function s(u){let h=u.target;h.index!==null&&e.remove(h.index);for(let x in h.attributes)e.remove(h.attributes[x]);for(let x in h.morphAttributes){let g=h.morphAttributes[x];for(let m=0,f=g.length;m<f;m++)e.remove(g[m])}h.removeEventListener("dispose",s),delete n[h.id];let p=o.get(h);p&&(e.remove(p),o.delete(h)),i.releaseStatesOfGeometry(h),h.isInstancedBufferGeometry===!0&&delete h._maxInstanceCount,t.memory.geometries--}function a(u,h){return n[h.id]===!0||(h.addEventListener("dispose",s),n[h.id]=!0,t.memory.geometries++),h}function c(u){let h=u.attributes;for(let x in h)e.update(h[x],r.ARRAY_BUFFER);let p=u.morphAttributes;for(let x in p){let g=p[x];for(let m=0,f=g.length;m<f;m++)e.update(g[m],r.ARRAY_BUFFER)}}function l(u){let h=[],p=u.index,x=u.attributes.position,g=0;if(p!==null){let S=p.array;g=p.version;for(let _=0,R=S.length;_<R;_+=3){let P=S[_+0],w=S[_+1],T=S[_+2];h.push(P,w,w,T,T,P)}}else if(x!==void 0){let S=x.array;g=x.version;for(let _=0,R=S.length/3-1;_<R;_+=3){let P=_+0,w=_+1,T=_+2;h.push(P,w,w,T,T,P)}}else return;let m=new(as(h)?on:nn)(h,1);m.version=g;let f=o.get(u);f&&e.remove(f),o.set(u,m)}function d(u){let h=o.get(u);if(h){let p=u.index;p!==null&&h.version<p.version&&l(u)}else l(u);return o.get(u)}return{get:a,update:c,getWireframeAttribute:d}}function dp(r,e,t){let i;function n(h){i=h}let o,s;function a(h){o=h.type,s=h.bytesPerElement}function c(h,p){r.drawElements(i,p,o,h*s),t.update(p,i,1)}function l(h,p,x){x!==0&&(r.drawElementsInstanced(i,p,o,h*s,x),t.update(p,i,x))}function d(h,p,x){if(x===0)return;let g=e.get("WEBGL_multi_draw");if(g===null)for(let m=0;m<x;m++)this.render(h[m]/s,p[m]);else{g.multiDrawElementsWEBGL(i,p,0,o,h,0,x);let m=0;for(let f=0;f<x;f++)m+=p[f];t.update(m,i,1)}}function u(h,p,x,g){if(x===0)return;let m=e.get("WEBGL_multi_draw");if(m===null)for(let f=0;f<h.length;f++)l(h[f]/s,p[f],g[f]);else{m.multiDrawElementsInstancedWEBGL(i,p,0,o,h,0,g,0,x);let f=0;for(let S=0;S<x;S++)f+=p[S];for(let S=0;S<g.length;S++)t.update(f,i,g[S])}}this.setMode=n,this.setIndex=a,this.render=c,this.renderInstances=l,this.renderMultiDraw=d,this.renderMultiDrawInstances=u}function up(r){let e={geometries:0,textures:0},t={frame:0,calls:0,triangles:0,points:0,lines:0};function i(o,s,a){switch(t.calls++,s){case r.TRIANGLES:t.triangles+=a*(o/3);break;case r.LINES:t.lines+=a*(o/2);break;case r.LINE_STRIP:t.lines+=a*(o-1);break;case r.LINE_LOOP:t.lines+=a*o;break;case r.POINTS:t.points+=a*o;break;default:console.error("THREE.WebGLInfo: Unknown draw mode:",s);break}}function n(){t.calls=0,t.triangles=0,t.points=0,t.lines=0}return{memory:e,render:t,programs:null,autoReset:!0,reset:n,update:i}}var Mn=class extends St{constructor(e=null,t=1,i=1,n=1){super(null),this.isDataArrayTexture=!0,this.image={data:e,width:t,height:i,depth:n},this.magFilter=nt,this.minFilter=nt,this.wrapR=Yt,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}};function hp(r,e,t){let i=new WeakMap,n=new ct;function o(s,a,c){let l=s.morphTargetInfluences,d=a.morphAttributes.position||a.morphAttributes.normal||a.morphAttributes.color,u=d!==void 0?d.length:0,h=i.get(a);if(h===void 0||h.count!==u){let y=function(){T.dispose(),i.delete(a),a.removeEventListener("dispose",y)};h!==void 0&&h.texture.dispose();let p=a.morphAttributes.position!==void 0,x=a.morphAttributes.normal!==void 0,g=a.morphAttributes.color!==void 0,m=a.morphAttributes.position||[],f=a.morphAttributes.normal||[],S=a.morphAttributes.color||[],_=0;p===!0&&(_=1),x===!0&&(_=2),g===!0&&(_=3);let R=a.attributes.position.count*_,P=1;R>e.maxTextureSize&&(P=Math.ceil(R/e.maxTextureSize),R=e.maxTextureSize);let w=new Float32Array(R*P*4*u),T=new Mn(w,R,P,u);T.type=oi,T.needsUpdate=!0;let C=_*4;for(let v=0;v<u;v++){let I=m[v],F=f[v],E=S[v],N=R*P*4*v;for(let W=0;W<I.count;W++){let X=W*C;p===!0&&(n.fromBufferAttribute(I,W),w[N+X+0]=n.x,w[N+X+1]=n.y,w[N+X+2]=n.z,w[N+X+3]=0),x===!0&&(n.fromBufferAttribute(F,W),w[N+X+4]=n.x,w[N+X+5]=n.y,w[N+X+6]=n.z,w[N+X+7]=0),g===!0&&(n.fromBufferAttribute(E,W),w[N+X+8]=n.x,w[N+X+9]=n.y,w[N+X+10]=n.z,w[N+X+11]=E.itemSize===4?n.w:1)}}h={count:u,texture:T,size:new be(R,P)},i.set(a,h),a.addEventListener("dispose",y)}if(s.isInstancedMesh===!0&&s.morphTexture!==null)c.getUniforms().setValue(r,"morphTexture",s.morphTexture,t);else{let p=0;for(let g=0;g<l.length;g++)p+=l[g];let x=a.morphTargetsRelative?1:1-p;c.getUniforms().setValue(r,"morphTargetBaseInfluence",x),c.getUniforms().setValue(r,"morphTargetInfluences",l)}c.getUniforms().setValue(r,"morphTargetsTexture",h.texture,t),c.getUniforms().setValue(r,"morphTargetsTextureSize",h.size)}return{update:o}}function fp(r,e,t,i){let n=new WeakMap;function o(c){let l=i.render.frame,d=c.geometry,u=e.get(c,d);if(n.get(u)!==l&&(e.update(u),n.set(u,l)),c.isInstancedMesh&&(c.hasEventListener("dispose",a)===!1&&c.addEventListener("dispose",a),n.get(c)!==l&&(t.update(c.instanceMatrix,r.ARRAY_BUFFER),c.instanceColor!==null&&t.update(c.instanceColor,r.ARRAY_BUFFER),n.set(c,l))),c.isSkinnedMesh){let h=c.skeleton;n.get(h)!==l&&(h.update(),n.set(h,l))}return u}function s(){n=new WeakMap}function a(c){let l=c.target;l.removeEventListener("dispose",a),t.remove(l.instanceMatrix),l.instanceColor!==null&&t.remove(l.instanceColor)}return{update:o,dispose:s}}var Ns=class extends St{constructor(e=null,t=1,i=1,n=1){super(null),this.isData3DTexture=!0,this.image={data:e,width:t,height:i,depth:n},this.magFilter=nt,this.minFilter=nt,this.wrapR=Yt,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}};var wn=class extends St{constructor(e,t,i,n,o,s,a,c,l,d){if(d=d!==void 0?d:Oi,d!==Oi&&d!==$i)throw new Error("DepthTexture format must be either THREE.DepthFormat or THREE.DepthStencilFormat");i===void 0&&d===Oi&&(i=yi),i===void 0&&d===$i&&(i=Fi),super(null,n,o,s,a,c,d,i,l),this.isDepthTexture=!0,this.image={width:e,height:t},this.magFilter=a!==void 0?a:nt,this.minFilter=c!==void 0?c:nt,this.flipY=!1,this.generateMipmaps=!1,this.compareFunction=null}copy(e){return super.copy(e),this.compareFunction=e.compareFunction,this}toJSON(e){let t=super.toJSON(e);return this.compareFunction!==null&&(t.compareFunction=this.compareFunction),t}};var yp=new St,Sp=new wn(1,1);Sp.compareFunction=ss;var bp=new Mn,Rp=new Ns,Mp=new Sn,pp=[],mp=[],gp=new Float32Array(16),xp=new Float32Array(9),vp=new Float32Array(4);function Tn(r,e,t){let i=r[0];if(i<=0||i>0)return r;let n=e*t,o=pp[n];if(o===void 0&&(o=new Float32Array(n),pp[n]=o),e!==0){i.toArray(o,0);for(let s=1,a=0;s!==e;++s)a+=t,r[s].toArray(o,a)}return o}function Ct(r,e){if(r.length!==e.length)return!1;for(let t=0,i=r.length;t<i;t++)if(r[t]!==e[t])return!1;return!0}function Pt(r,e){for(let t=0,i=e.length;t<i;t++)r[t]=e[t]}function Bs(r,e){let t=mp[e];t===void 0&&(t=new Int32Array(e),mp[e]=t);for(let i=0;i!==e;++i)t[i]=r.allocateTextureUnit();return t}function ex(r,e){let t=this.cache;t[0]!==e&&(r.uniform1f(this.addr,e),t[0]=e)}function tx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(r.uniform2f(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ct(t,e))return;r.uniform2fv(this.addr,e),Pt(t,e)}}function ix(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(r.uniform3f(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else if(e.r!==void 0)(t[0]!==e.r||t[1]!==e.g||t[2]!==e.b)&&(r.uniform3f(this.addr,e.r,e.g,e.b),t[0]=e.r,t[1]=e.g,t[2]=e.b);else{if(Ct(t,e))return;r.uniform3fv(this.addr,e),Pt(t,e)}}function rx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(r.uniform4f(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ct(t,e))return;r.uniform4fv(this.addr,e),Pt(t,e)}}function nx(r,e){let t=this.cache,i=e.elements;if(i===void 0){if(Ct(t,e))return;r.uniformMatrix2fv(this.addr,!1,e),Pt(t,e)}else{if(Ct(t,i))return;vp.set(i),r.uniformMatrix2fv(this.addr,!1,vp),Pt(t,i)}}function ox(r,e){let t=this.cache,i=e.elements;if(i===void 0){if(Ct(t,e))return;r.uniformMatrix3fv(this.addr,!1,e),Pt(t,e)}else{if(Ct(t,i))return;xp.set(i),r.uniformMatrix3fv(this.addr,!1,xp),Pt(t,i)}}function sx(r,e){let t=this.cache,i=e.elements;if(i===void 0){if(Ct(t,e))return;r.uniformMatrix4fv(this.addr,!1,e),Pt(t,e)}else{if(Ct(t,i))return;gp.set(i),r.uniformMatrix4fv(this.addr,!1,gp),Pt(t,i)}}function ax(r,e){let t=this.cache;t[0]!==e&&(r.uniform1i(this.addr,e),t[0]=e)}function cx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(r.uniform2i(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ct(t,e))return;r.uniform2iv(this.addr,e),Pt(t,e)}}function lx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(r.uniform3i(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else{if(Ct(t,e))return;r.uniform3iv(this.addr,e),Pt(t,e)}}function dx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(r.uniform4i(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ct(t,e))return;r.uniform4iv(this.addr,e),Pt(t,e)}}function ux(r,e){let t=this.cache;t[0]!==e&&(r.uniform1ui(this.addr,e),t[0]=e)}function hx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(r.uniform2ui(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ct(t,e))return;r.uniform2uiv(this.addr,e),Pt(t,e)}}function fx(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(r.uniform3ui(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else{if(Ct(t,e))return;r.uniform3uiv(this.addr,e),Pt(t,e)}}function px(r,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(r.uniform4ui(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ct(t,e))return;r.uniform4uiv(this.addr,e),Pt(t,e)}}function mx(r,e,t){let i=this.cache,n=t.allocateTextureUnit();i[0]!==n&&(r.uniform1i(this.addr,n),i[0]=n);let o=this.type===r.SAMPLER_2D_SHADOW?Sp:yp;t.setTexture2D(e||o,n)}function gx(r,e,t){let i=this.cache,n=t.allocateTextureUnit();i[0]!==n&&(r.uniform1i(this.addr,n),i[0]=n),t.setTexture3D(e||Rp,n)}function xx(r,e,t){let i=this.cache,n=t.allocateTextureUnit();i[0]!==n&&(r.uniform1i(this.addr,n),i[0]=n),t.setTextureCube(e||Mp,n)}function vx(r,e,t){let i=this.cache,n=t.allocateTextureUnit();i[0]!==n&&(r.uniform1i(this.addr,n),i[0]=n),t.setTexture2DArray(e||bp,n)}function _x(r){switch(r){case 5126:return ex;case 35664:return tx;case 35665:return ix;case 35666:return rx;case 35674:return nx;case 35675:return ox;case 35676:return sx;case 5124:case 35670:return ax;case 35667:case 35671:return cx;case 35668:case 35672:return lx;case 35669:case 35673:return dx;case 5125:return ux;case 36294:return hx;case 36295:return fx;case 36296:return px;case 35678:case 36198:case 36298:case 36306:case 35682:return mx;case 35679:case 36299:case 36307:return gx;case 35680:case 36300:case 36308:case 36293:return xx;case 36289:case 36303:case 36311:case 36292:return vx}}function yx(r,e){r.uniform1fv(this.addr,e)}function Sx(r,e){let t=Tn(e,this.size,2);r.uniform2fv(this.addr,t)}function bx(r,e){let t=Tn(e,this.size,3);r.uniform3fv(this.addr,t)}function Rx(r,e){let t=Tn(e,this.size,4);r.uniform4fv(this.addr,t)}function Mx(r,e){let t=Tn(e,this.size,4);r.uniformMatrix2fv(this.addr,!1,t)}function wx(r,e){let t=Tn(e,this.size,9);r.uniformMatrix3fv(this.addr,!1,t)}function Tx(r,e){let t=Tn(e,this.size,16);r.uniformMatrix4fv(this.addr,!1,t)}function Ex(r,e){r.uniform1iv(this.addr,e)}function Ax(r,e){r.uniform2iv(this.addr,e)}function Cx(r,e){r.uniform3iv(this.addr,e)}function Px(r,e){r.uniform4iv(this.addr,e)}function Ix(r,e){r.uniform1uiv(this.addr,e)}function Dx(r,e){r.uniform2uiv(this.addr,e)}function Lx(r,e){r.uniform3uiv(this.addr,e)}function Ux(r,e){r.uniform4uiv(this.addr,e)}function Fx(r,e,t){let i=this.cache,n=e.length,o=Bs(t,n);Ct(i,o)||(r.uniform1iv(this.addr,o),Pt(i,o));for(let s=0;s!==n;++s)t.setTexture2D(e[s]||yp,o[s])}function Ox(r,e,t){let i=this.cache,n=e.length,o=Bs(t,n);Ct(i,o)||(r.uniform1iv(this.addr,o),Pt(i,o));for(let s=0;s!==n;++s)t.setTexture3D(e[s]||Rp,o[s])}function Nx(r,e,t){let i=this.cache,n=e.length,o=Bs(t,n);Ct(i,o)||(r.uniform1iv(this.addr,o),Pt(i,o));for(let s=0;s!==n;++s)t.setTextureCube(e[s]||Mp,o[s])}function Bx(r,e,t){let i=this.cache,n=e.length,o=Bs(t,n);Ct(i,o)||(r.uniform1iv(this.addr,o),Pt(i,o));for(let s=0;s!==n;++s)t.setTexture2DArray(e[s]||bp,o[s])}function kx(r){switch(r){case 5126:return yx;case 35664:return Sx;case 35665:return bx;case 35666:return Rx;case 35674:return Mx;case 35675:return wx;case 35676:return Tx;case 5124:case 35670:return Ex;case 35667:case 35671:return Ax;case 35668:case 35672:return Cx;case 35669:case 35673:return Px;case 5125:return Ix;case 36294:return Dx;case 36295:return Lx;case 36296:return Ux;case 35678:case 36198:case 36298:case 36306:case 35682:return Fx;case 35679:case 36299:case 36307:return Ox;case 35680:case 36300:case 36308:case 36293:return Nx;case 36289:case 36303:case 36311:case 36292:return Bx}}var bc=class{constructor(e,t,i){this.id=e,this.addr=i,this.cache=[],this.type=t.type,this.setValue=_x(t.type)}},Rc=class{constructor(e,t,i){this.id=e,this.addr=i,this.cache=[],this.type=t.type,this.size=t.size,this.setValue=kx(t.type)}},Mc=class{constructor(e){this.id=e,this.seq=[],this.map={}}setValue(e,t,i){let n=this.seq;for(let o=0,s=n.length;o!==s;++o){let a=n[o];a.setValue(e,t[a.id],i)}}},Sc=/(\w+)(\])?(\[|\.)?/g;function _p(r,e){r.seq.push(e),r.map[e.id]=e}function zx(r,e,t){let i=r.name,n=i.length;for(Sc.lastIndex=0;;){let o=Sc.exec(i),s=Sc.lastIndex,a=o[1],c=o[2]==="]",l=o[3];if(c&&(a=a|0),l===void 0||l==="["&&s+2===n){_p(t,l===void 0?new bc(a,r,e):new Rc(a,r,e));break}else{let u=t.map[a];u===void 0&&(u=new Mc(a),_p(t,u)),t=u}}}var Sr=class{constructor(e,t){this.seq=[],this.map={};let i=e.getProgramParameter(t,e.ACTIVE_UNIFORMS);for(let n=0;n<i;++n){let o=e.getActiveUniform(t,n),s=e.getUniformLocation(t,o.name);zx(o,s,this)}}setValue(e,t,i,n){let o=this.map[t];o!==void 0&&o.setValue(e,i,n)}setOptional(e,t,i){let n=t[i];n!==void 0&&this.setValue(e,i,n)}static upload(e,t,i,n){for(let o=0,s=t.length;o!==s;++o){let a=t[o],c=i[a.id];c.needsUpdate!==!1&&a.setValue(e,c.value,n)}}static seqWithValue(e,t){let i=[];for(let n=0,o=e.length;n!==o;++n){let s=e[n];s.id in t&&i.push(s)}return i}};function wc(r,e,t){let i=r.createShader(e);return r.shaderSource(i,t),r.compileShader(i),i}var Vx=37297,Gx=0;function Wx(r,e){let t=r.split(`
`),i=[],n=Math.max(e-6,0),o=Math.min(e+6,t.length);for(let s=n;s<o;s++){let a=s+1;i.push(`${a===e?">":" "} ${a}: ${t[s]}`)}return i.join(`
`)}function Hx(r){let e=Oe.getPrimaries(Oe.workingColorSpace),t=Oe.getPrimaries(r),i;switch(e===t?i="":e===rn&&t===tn?i="LinearDisplayP3ToLinearSRGB":e===tn&&t===rn&&(i="LinearSRGBToLinearDisplayP3"),r){case ei:case Ir:return[i,"LinearTransferOETF"];case At:case Qr:return[i,"sRGBTransferOETF"];default:return console.warn("THREE.WebGLProgram: Unsupported color space:",r),[i,"LinearTransferOETF"]}}function wp(r,e,t){let i=r.getShaderParameter(e,r.COMPILE_STATUS),n=r.getShaderInfoLog(e).trim();if(i&&n==="")return"";let o=/ERROR: 0:(\d+)/.exec(n);if(o){let s=parseInt(o[1]);return t.toUpperCase()+`

`+n+`

`+Wx(r.getShaderSource(e),s)}else return n}function jx(r,e){let t=Hx(e);return`vec4 ${r}( vec4 value ) { return ${t[0]}( ${t[1]}( value ) ); }`}function Xx(r,e){let t;switch(e){case hd:t="Linear";break;case fd:t="Reinhard";break;case pd:t="OptimizedCineon";break;case md:t="ACESFilmic";break;case xd:t="AgX";break;case vd:t="Neutral";break;case gd:t="Custom";break;default:console.warn("THREE.WebGLProgram: Unsupported toneMapping:",e),t="Linear"}return"vec3 "+r+"( vec3 color ) { return "+t+"ToneMapping( color ); }"}function qx(r){return[r.extensionClipCullDistance?"#extension GL_ANGLE_clip_cull_distance : require":"",r.extensionMultiDraw?"#extension GL_ANGLE_multi_draw : require":""].filter(po).join(`
`)}function $x(r){let e=[];for(let t in r){let i=r[t];i!==!1&&e.push("#define "+t+" "+i)}return e.join(`
`)}function Yx(r,e){let t={},i=r.getProgramParameter(e,r.ACTIVE_ATTRIBUTES);for(let n=0;n<i;n++){let o=r.getActiveAttrib(e,n),s=o.name,a=1;o.type===r.FLOAT_MAT2&&(a=2),o.type===r.FLOAT_MAT3&&(a=3),o.type===r.FLOAT_MAT4&&(a=4),t[s]={type:o.type,location:r.getAttribLocation(e,s),locationSize:a}}return t}function po(r){return r!==""}function Tp(r,e){let t=e.numSpotLightShadows+e.numSpotLightMaps-e.numSpotLightShadowsWithMaps;return r.replace(/NUM_DIR_LIGHTS/g,e.numDirLights).replace(/NUM_SPOT_LIGHTS/g,e.numSpotLights).replace(/NUM_SPOT_LIGHT_MAPS/g,e.numSpotLightMaps).replace(/NUM_SPOT_LIGHT_COORDS/g,t).replace(/NUM_RECT_AREA_LIGHTS/g,e.numRectAreaLights).replace(/NUM_POINT_LIGHTS/g,e.numPointLights).replace(/NUM_HEMI_LIGHTS/g,e.numHemiLights).replace(/NUM_DIR_LIGHT_SHADOWS/g,e.numDirLightShadows).replace(/NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS/g,e.numSpotLightShadowsWithMaps).replace(/NUM_SPOT_LIGHT_SHADOWS/g,e.numSpotLightShadows).replace(/NUM_POINT_LIGHT_SHADOWS/g,e.numPointLightShadows)}function Ep(r,e){return r.replace(/NUM_CLIPPING_PLANES/g,e.numClippingPlanes).replace(/UNION_CLIPPING_PLANES/g,e.numClippingPlanes-e.numClipIntersection)}var Kx=/^[ \t]*#include +<([\w\d./]+)>/gm;function Tc(r){return r.replace(Kx,Jx)}var Zx=new Map;function Jx(r,e){let t=Ae[e];if(t===void 0){let i=Zx.get(e);if(i!==void 0)t=Ae[i],console.warn('THREE.WebGLRenderer: Shader chunk "%s" has been deprecated. Use "%s" instead.',e,i);else throw new Error("Can not resolve #include <"+e+">")}return Tc(t)}var Qx=/#pragma unroll_loop_start\s+for\s*\(\s*int\s+i\s*=\s*(\d+)\s*;\s*i\s*<\s*(\d+)\s*;\s*i\s*\+\+\s*\)\s*{([\s\S]+?)}\s+#pragma unroll_loop_end/g;function Ap(r){return r.replace(Qx,ev)}function ev(r,e,t,i){let n="";for(let o=parseInt(e);o<parseInt(t);o++)n+=i.replace(/\[\s*i\s*\]/g,"[ "+o+" ]").replace(/UNROLLED_LOOP_INDEX/g,o);return n}function Cp(r){let e=`precision ${r.precision} float;
	precision ${r.precision} int;
	precision ${r.precision} sampler2D;
	precision ${r.precision} samplerCube;
	precision ${r.precision} sampler3D;
	precision ${r.precision} sampler2DArray;
	precision ${r.precision} sampler2DShadow;
	precision ${r.precision} samplerCubeShadow;
	precision ${r.precision} sampler2DArrayShadow;
	precision ${r.precision} isampler2D;
	precision ${r.precision} isampler3D;
	precision ${r.precision} isamplerCube;
	precision ${r.precision} isampler2DArray;
	precision ${r.precision} usampler2D;
	precision ${r.precision} usampler3D;
	precision ${r.precision} usamplerCube;
	precision ${r.precision} usampler2DArray;
	`;return r.precision==="highp"?e+=`
#define HIGH_PRECISION`:r.precision==="mediump"?e+=`
#define MEDIUM_PRECISION`:r.precision==="lowp"&&(e+=`
#define LOW_PRECISION`),e}function tv(r){let e="SHADOWMAP_TYPE_BASIC";return r.shadowMapType===jo?e="SHADOWMAP_TYPE_PCF":r.shadowMapType===Jn?e="SHADOWMAP_TYPE_PCF_SOFT":r.shadowMapType===xi&&(e="SHADOWMAP_TYPE_VSM"),e}function iv(r){let e="ENVMAP_TYPE_CUBE";if(r.envMap)switch(r.envMapMode){case vi:case Ui:e="ENVMAP_TYPE_CUBE";break;case hr:e="ENVMAP_TYPE_CUBE_UV";break}return e}function rv(r){let e="ENVMAP_MODE_REFLECTION";return r.envMap&&r.envMapMode===Ui&&(e="ENVMAP_MODE_REFRACTION"),e}function nv(r){let e="ENVMAP_BLENDING_NONE";if(r.envMap)switch(r.combine){case Xo:e="ENVMAP_BLENDING_MULTIPLY";break;case dd:e="ENVMAP_BLENDING_MIX";break;case ud:e="ENVMAP_BLENDING_ADD";break}return e}function ov(r){let e=r.envMapCubeUVHeight;if(e===null)return null;let t=Math.log2(e)-2,i=1/e;return{texelWidth:1/(3*Math.max(Math.pow(2,t),112)),texelHeight:i,maxMip:t}}function Pp(r,e,t,i){let n=r.getContext(),o=t.defines,s=t.vertexShader,a=t.fragmentShader,c=tv(t),l=iv(t),d=rv(t),u=nv(t),h=ov(t),p=qx(t),x=$x(o),g=n.createProgram(),m,f,S=t.glslVersion?"#version "+t.glslVersion+`
`:"";t.isRawShaderMaterial?(m=["#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,x].filter(po).join(`
`),m.length>0&&(m+=`
`),f=["#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,x].filter(po).join(`
`),f.length>0&&(f+=`
`)):(m=[Cp(t),"#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,x,t.extensionClipCullDistance?"#define USE_CLIP_DISTANCE":"",t.batching?"#define USE_BATCHING":"",t.instancing?"#define USE_INSTANCING":"",t.instancingColor?"#define USE_INSTANCING_COLOR":"",t.instancingMorph?"#define USE_INSTANCING_MORPH":"",t.useFog&&t.fog?"#define USE_FOG":"",t.useFog&&t.fogExp2?"#define FOG_EXP2":"",t.map?"#define USE_MAP":"",t.envMap?"#define USE_ENVMAP":"",t.envMap?"#define "+d:"",t.lightMap?"#define USE_LIGHTMAP":"",t.aoMap?"#define USE_AOMAP":"",t.bumpMap?"#define USE_BUMPMAP":"",t.normalMap?"#define USE_NORMALMAP":"",t.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",t.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",t.displacementMap?"#define USE_DISPLACEMENTMAP":"",t.emissiveMap?"#define USE_EMISSIVEMAP":"",t.anisotropy?"#define USE_ANISOTROPY":"",t.anisotropyMap?"#define USE_ANISOTROPYMAP":"",t.clearcoatMap?"#define USE_CLEARCOATMAP":"",t.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",t.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",t.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",t.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",t.specularMap?"#define USE_SPECULARMAP":"",t.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",t.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",t.roughnessMap?"#define USE_ROUGHNESSMAP":"",t.metalnessMap?"#define USE_METALNESSMAP":"",t.alphaMap?"#define USE_ALPHAMAP":"",t.alphaHash?"#define USE_ALPHAHASH":"",t.transmission?"#define USE_TRANSMISSION":"",t.transmissionMap?"#define USE_TRANSMISSIONMAP":"",t.thicknessMap?"#define USE_THICKNESSMAP":"",t.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",t.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",t.mapUv?"#define MAP_UV "+t.mapUv:"",t.alphaMapUv?"#define ALPHAMAP_UV "+t.alphaMapUv:"",t.lightMapUv?"#define LIGHTMAP_UV "+t.lightMapUv:"",t.aoMapUv?"#define AOMAP_UV "+t.aoMapUv:"",t.emissiveMapUv?"#define EMISSIVEMAP_UV "+t.emissiveMapUv:"",t.bumpMapUv?"#define BUMPMAP_UV "+t.bumpMapUv:"",t.normalMapUv?"#define NORMALMAP_UV "+t.normalMapUv:"",t.displacementMapUv?"#define DISPLACEMENTMAP_UV "+t.displacementMapUv:"",t.metalnessMapUv?"#define METALNESSMAP_UV "+t.metalnessMapUv:"",t.roughnessMapUv?"#define ROUGHNESSMAP_UV "+t.roughnessMapUv:"",t.anisotropyMapUv?"#define ANISOTROPYMAP_UV "+t.anisotropyMapUv:"",t.clearcoatMapUv?"#define CLEARCOATMAP_UV "+t.clearcoatMapUv:"",t.clearcoatNormalMapUv?"#define CLEARCOAT_NORMALMAP_UV "+t.clearcoatNormalMapUv:"",t.clearcoatRoughnessMapUv?"#define CLEARCOAT_ROUGHNESSMAP_UV "+t.clearcoatRoughnessMapUv:"",t.iridescenceMapUv?"#define IRIDESCENCEMAP_UV "+t.iridescenceMapUv:"",t.iridescenceThicknessMapUv?"#define IRIDESCENCE_THICKNESSMAP_UV "+t.iridescenceThicknessMapUv:"",t.sheenColorMapUv?"#define SHEEN_COLORMAP_UV "+t.sheenColorMapUv:"",t.sheenRoughnessMapUv?"#define SHEEN_ROUGHNESSMAP_UV "+t.sheenRoughnessMapUv:"",t.specularMapUv?"#define SPECULARMAP_UV "+t.specularMapUv:"",t.specularColorMapUv?"#define SPECULAR_COLORMAP_UV "+t.specularColorMapUv:"",t.specularIntensityMapUv?"#define SPECULAR_INTENSITYMAP_UV "+t.specularIntensityMapUv:"",t.transmissionMapUv?"#define TRANSMISSIONMAP_UV "+t.transmissionMapUv:"",t.thicknessMapUv?"#define THICKNESSMAP_UV "+t.thicknessMapUv:"",t.vertexTangents&&t.flatShading===!1?"#define USE_TANGENT":"",t.vertexColors?"#define USE_COLOR":"",t.vertexAlphas?"#define USE_COLOR_ALPHA":"",t.vertexUv1s?"#define USE_UV1":"",t.vertexUv2s?"#define USE_UV2":"",t.vertexUv3s?"#define USE_UV3":"",t.pointsUvs?"#define USE_POINTS_UV":"",t.flatShading?"#define FLAT_SHADED":"",t.skinning?"#define USE_SKINNING":"",t.morphTargets?"#define USE_MORPHTARGETS":"",t.morphNormals&&t.flatShading===!1?"#define USE_MORPHNORMALS":"",t.morphColors?"#define USE_MORPHCOLORS":"",t.morphTargetsCount>0?"#define MORPHTARGETS_TEXTURE":"",t.morphTargetsCount>0?"#define MORPHTARGETS_TEXTURE_STRIDE "+t.morphTextureStride:"",t.morphTargetsCount>0?"#define MORPHTARGETS_COUNT "+t.morphTargetsCount:"",t.doubleSided?"#define DOUBLE_SIDED":"",t.flipSided?"#define FLIP_SIDED":"",t.shadowMapEnabled?"#define USE_SHADOWMAP":"",t.shadowMapEnabled?"#define "+c:"",t.sizeAttenuation?"#define USE_SIZEATTENUATION":"",t.numLightProbes>0?"#define USE_LIGHT_PROBES":"",t.useLegacyLights?"#define LEGACY_LIGHTS":"",t.logarithmicDepthBuffer?"#define USE_LOGDEPTHBUF":"","uniform mat4 modelMatrix;","uniform mat4 modelViewMatrix;","uniform mat4 projectionMatrix;","uniform mat4 viewMatrix;","uniform mat3 normalMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;","#ifdef USE_INSTANCING","	attribute mat4 instanceMatrix;","#endif","#ifdef USE_INSTANCING_COLOR","	attribute vec3 instanceColor;","#endif","#ifdef USE_INSTANCING_MORPH","	uniform sampler2D morphTexture;","#endif","attribute vec3 position;","attribute vec3 normal;","attribute vec2 uv;","#ifdef USE_UV1","	attribute vec2 uv1;","#endif","#ifdef USE_UV2","	attribute vec2 uv2;","#endif","#ifdef USE_UV3","	attribute vec2 uv3;","#endif","#ifdef USE_TANGENT","	attribute vec4 tangent;","#endif","#if defined( USE_COLOR_ALPHA )","	attribute vec4 color;","#elif defined( USE_COLOR )","	attribute vec3 color;","#endif","#if ( defined( USE_MORPHTARGETS ) && ! defined( MORPHTARGETS_TEXTURE ) )","	attribute vec3 morphTarget0;","	attribute vec3 morphTarget1;","	attribute vec3 morphTarget2;","	attribute vec3 morphTarget3;","	#ifdef USE_MORPHNORMALS","		attribute vec3 morphNormal0;","		attribute vec3 morphNormal1;","		attribute vec3 morphNormal2;","		attribute vec3 morphNormal3;","	#else","		attribute vec3 morphTarget4;","		attribute vec3 morphTarget5;","		attribute vec3 morphTarget6;","		attribute vec3 morphTarget7;","	#endif","#endif","#ifdef USE_SKINNING","	attribute vec4 skinIndex;","	attribute vec4 skinWeight;","#endif",`
`].filter(po).join(`
`),f=[Cp(t),"#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,x,t.useFog&&t.fog?"#define USE_FOG":"",t.useFog&&t.fogExp2?"#define FOG_EXP2":"",t.alphaToCoverage?"#define ALPHA_TO_COVERAGE":"",t.map?"#define USE_MAP":"",t.matcap?"#define USE_MATCAP":"",t.envMap?"#define USE_ENVMAP":"",t.envMap?"#define "+l:"",t.envMap?"#define "+d:"",t.envMap?"#define "+u:"",h?"#define CUBEUV_TEXEL_WIDTH "+h.texelWidth:"",h?"#define CUBEUV_TEXEL_HEIGHT "+h.texelHeight:"",h?"#define CUBEUV_MAX_MIP "+h.maxMip+".0":"",t.lightMap?"#define USE_LIGHTMAP":"",t.aoMap?"#define USE_AOMAP":"",t.bumpMap?"#define USE_BUMPMAP":"",t.normalMap?"#define USE_NORMALMAP":"",t.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",t.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",t.emissiveMap?"#define USE_EMISSIVEMAP":"",t.anisotropy?"#define USE_ANISOTROPY":"",t.anisotropyMap?"#define USE_ANISOTROPYMAP":"",t.clearcoat?"#define USE_CLEARCOAT":"",t.clearcoatMap?"#define USE_CLEARCOATMAP":"",t.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",t.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",t.dispersion?"#define USE_DISPERSION":"",t.iridescence?"#define USE_IRIDESCENCE":"",t.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",t.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",t.specularMap?"#define USE_SPECULARMAP":"",t.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",t.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",t.roughnessMap?"#define USE_ROUGHNESSMAP":"",t.metalnessMap?"#define USE_METALNESSMAP":"",t.alphaMap?"#define USE_ALPHAMAP":"",t.alphaTest?"#define USE_ALPHATEST":"",t.alphaHash?"#define USE_ALPHAHASH":"",t.sheen?"#define USE_SHEEN":"",t.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",t.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",t.transmission?"#define USE_TRANSMISSION":"",t.transmissionMap?"#define USE_TRANSMISSIONMAP":"",t.thicknessMap?"#define USE_THICKNESSMAP":"",t.vertexTangents&&t.flatShading===!1?"#define USE_TANGENT":"",t.vertexColors||t.instancingColor?"#define USE_COLOR":"",t.vertexAlphas?"#define USE_COLOR_ALPHA":"",t.vertexUv1s?"#define USE_UV1":"",t.vertexUv2s?"#define USE_UV2":"",t.vertexUv3s?"#define USE_UV3":"",t.pointsUvs?"#define USE_POINTS_UV":"",t.gradientMap?"#define USE_GRADIENTMAP":"",t.flatShading?"#define FLAT_SHADED":"",t.doubleSided?"#define DOUBLE_SIDED":"",t.flipSided?"#define FLIP_SIDED":"",t.shadowMapEnabled?"#define USE_SHADOWMAP":"",t.shadowMapEnabled?"#define "+c:"",t.premultipliedAlpha?"#define PREMULTIPLIED_ALPHA":"",t.numLightProbes>0?"#define USE_LIGHT_PROBES":"",t.useLegacyLights?"#define LEGACY_LIGHTS":"",t.decodeVideoTexture?"#define DECODE_VIDEO_TEXTURE":"",t.logarithmicDepthBuffer?"#define USE_LOGDEPTHBUF":"","uniform mat4 viewMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;",t.toneMapping!==ui?"#define TONE_MAPPING":"",t.toneMapping!==ui?Ae.tonemapping_pars_fragment:"",t.toneMapping!==ui?Xx("toneMapping",t.toneMapping):"",t.dithering?"#define DITHERING":"",t.opaque?"#define OPAQUE":"",Ae.colorspace_pars_fragment,jx("linearToOutputTexel",t.outputColorSpace),t.useDepthPacking?"#define DEPTH_PACKING "+t.depthPacking:"",`
`].filter(po).join(`
`)),s=Tc(s),s=Tp(s,t),s=Ep(s,t),a=Tc(a),a=Tp(a,t),a=Ep(a,t),s=Ap(s),a=Ap(a),t.isRawShaderMaterial!==!0&&(S=`#version 300 es
`,m=[p,"#define attribute in","#define varying out","#define texture2D texture"].join(`
`)+`
`+m,f=["#define varying in",t.glslVersion===Ja?"":"layout(location = 0) out highp vec4 pc_fragColor;",t.glslVersion===Ja?"":"#define gl_FragColor pc_fragColor","#define gl_FragDepthEXT gl_FragDepth","#define texture2D texture","#define textureCube texture","#define texture2DProj textureProj","#define texture2DLodEXT textureLod","#define texture2DProjLodEXT textureProjLod","#define textureCubeLodEXT textureLod","#define texture2DGradEXT textureGrad","#define texture2DProjGradEXT textureProjGrad","#define textureCubeGradEXT textureGrad"].join(`
`)+`
`+f);let _=S+m+s,R=S+f+a,P=wc(n,n.VERTEX_SHADER,_),w=wc(n,n.FRAGMENT_SHADER,R);n.attachShader(g,P),n.attachShader(g,w),t.index0AttributeName!==void 0?n.bindAttribLocation(g,0,t.index0AttributeName):t.morphTargets===!0&&n.bindAttribLocation(g,0,"position"),n.linkProgram(g);function T(I){if(r.debug.checkShaderErrors){let F=n.getProgramInfoLog(g).trim(),E=n.getShaderInfoLog(P).trim(),N=n.getShaderInfoLog(w).trim(),W=!0,X=!0;if(n.getProgramParameter(g,n.LINK_STATUS)===!1)if(W=!1,typeof r.debug.onShaderError=="function")r.debug.onShaderError(n,g,P,w);else{let te=wp(n,P,"vertex"),V=wp(n,w,"fragment");console.error("THREE.WebGLProgram: Shader Error "+n.getError()+" - VALIDATE_STATUS "+n.getProgramParameter(g,n.VALIDATE_STATUS)+`

Material Name: `+I.name+`
Material Type: `+I.type+`

Program Info Log: `+F+`
`+te+`
`+V)}else F!==""?console.warn("THREE.WebGLProgram: Program Info Log:",F):(E===""||N==="")&&(X=!1);X&&(I.diagnostics={runnable:W,programLog:F,vertexShader:{log:E,prefix:m},fragmentShader:{log:N,prefix:f}})}n.deleteShader(P),n.deleteShader(w),C=new Sr(n,g),y=Yx(n,g)}let C;this.getUniforms=function(){return C===void 0&&T(this),C};let y;this.getAttributes=function(){return y===void 0&&T(this),y};let v=t.rendererExtensionParallelShaderCompile===!1;return this.isReady=function(){return v===!1&&(v=n.getProgramParameter(g,Vx)),v},this.destroy=function(){i.releaseStatesOfProgram(this),n.deleteProgram(g),this.program=void 0},this.type=t.shaderType,this.name=t.shaderName,this.id=Gx++,this.cacheKey=e,this.usedTimes=1,this.program=g,this.vertexShader=P,this.fragmentShader=w,this}var sv=0,ks=class{constructor(){this.shaderCache=new Map,this.materialCache=new Map}update(e){let t=e.vertexShader,i=e.fragmentShader,n=this._getShaderStage(t),o=this._getShaderStage(i),s=this._getShaderCacheForMaterial(e);return s.has(n)===!1&&(s.add(n),n.usedTimes++),s.has(o)===!1&&(s.add(o),o.usedTimes++),this}remove(e){let t=this.materialCache.get(e);for(let i of t)i.usedTimes--,i.usedTimes===0&&this.shaderCache.delete(i.code);return this.materialCache.delete(e),this}getVertexShaderID(e){return this._getShaderStage(e.vertexShader).id}getFragmentShaderID(e){return this._getShaderStage(e.fragmentShader).id}dispose(){this.shaderCache.clear(),this.materialCache.clear()}_getShaderCacheForMaterial(e){let t=this.materialCache,i=t.get(e);return i===void 0&&(i=new Set,t.set(e,i)),i}_getShaderStage(e){let t=this.shaderCache,i=t.get(e);return i===void 0&&(i=new Ec(e),t.set(e,i)),i}},Ec=class{constructor(e){this.id=sv++,this.code=e,this.usedTimes=0}};function Ip(r,e,t,i,n,o,s){let a=new an,c=new ks,l=new Set,d=[],u=n.logarithmicDepthBuffer,h=n.vertexTextures,p=n.precision,x={MeshDepthMaterial:"depth",MeshDistanceMaterial:"distanceRGBA",MeshNormalMaterial:"normal",MeshBasicMaterial:"basic",MeshLambertMaterial:"lambert",MeshPhongMaterial:"phong",MeshToonMaterial:"toon",MeshStandardMaterial:"physical",MeshPhysicalMaterial:"physical",MeshMatcapMaterial:"matcap",LineBasicMaterial:"basic",LineDashedMaterial:"dashed",PointsMaterial:"points",ShadowMaterial:"shadow",SpriteMaterial:"sprite"};function g(y){return l.add(y),y===0?"uv":`uv${y}`}function m(y,v,I,F,E){let N=F.fog,W=E.geometry,X=y.isMeshStandardMaterial?F.environment:null,te=(y.isMeshStandardMaterial?t:e).get(y.envMap||X),V=te&&te.mapping===hr?te.image.height:null,J=x[y.type];y.precision!==null&&(p=n.getMaxPrecision(y.precision),p!==y.precision&&console.warn("THREE.WebGLProgram.getParameters:",y.precision,"not supported, using",p,"instead."));let Q=W.morphAttributes.position||W.morphAttributes.normal||W.morphAttributes.color,me=Q!==void 0?Q.length:0,Ne=0;W.morphAttributes.position!==void 0&&(Ne=1),W.morphAttributes.normal!==void 0&&(Ne=2),W.morphAttributes.color!==void 0&&(Ne=3);let tt,H,ee,he;if(J){let Ke=pi[J];tt=Ke.vertexShader,H=Ke.fragmentShader}else tt=y.vertexShader,H=y.fragmentShader,c.update(y),ee=c.getVertexShaderID(y),he=c.getFragmentShaderID(y);let ne=r.getRenderTarget(),Be=E.isInstancedMesh===!0,ke=E.isBatchedMesh===!0,O=!!y.map,ot=!!y.matcap,ve=!!te,it=!!y.aoMap,Re=!!y.lightMap,Ve=!!y.bumpMap,Ue=!!y.normalMap,Ge=!!y.displacementMap,pt=!!y.emissiveMap,A=!!y.metalnessMap,b=!!y.roughnessMap,G=y.anisotropy>0,q=y.clearcoat>0,Y=y.dispersion>0,K=y.iridescence>0,xe=y.sheen>0,ce=y.transmission>0,ae=G&&!!y.anisotropyMap,Ie=q&&!!y.clearcoatMap,re=q&&!!y.clearcoatNormalMap,ge=q&&!!y.clearcoatRoughnessMap,He=K&&!!y.iridescenceMap,_e=K&&!!y.iridescenceThicknessMap,de=xe&&!!y.sheenColorMap,De=xe&&!!y.sheenRoughnessMap,ze=!!y.specularMap,_t=!!y.specularColorMap,Le=!!y.specularIntensityMap,L=ce&&!!y.transmissionMap,$=ce&&!!y.thicknessMap,j=!!y.gradientMap,oe=!!y.alphaMap,le=y.alphaTest>0,je=!!y.alphaHash,st=!!y.extensions,mt=ui;y.toneMapped&&(ne===null||ne.isXRRenderTarget===!0)&&(mt=r.toneMapping);let kt={shaderID:J,shaderType:y.type,shaderName:y.name,vertexShader:tt,fragmentShader:H,defines:y.defines,customVertexShaderID:ee,customFragmentShaderID:he,isRawShaderMaterial:y.isRawShaderMaterial===!0,glslVersion:y.glslVersion,precision:p,batching:ke,instancing:Be,instancingColor:Be&&E.instanceColor!==null,instancingMorph:Be&&E.morphTexture!==null,supportsVertexTextures:h,outputColorSpace:ne===null?r.outputColorSpace:ne.isXRRenderTarget===!0?ne.texture.colorSpace:ei,alphaToCoverage:!!y.alphaToCoverage,map:O,matcap:ot,envMap:ve,envMapMode:ve&&te.mapping,envMapCubeUVHeight:V,aoMap:it,lightMap:Re,bumpMap:Ve,normalMap:Ue,displacementMap:h&&Ge,emissiveMap:pt,normalMapObjectSpace:Ue&&y.normalMapType===Ld,normalMapTangentSpace:Ue&&y.normalMapType===Dd,metalnessMap:A,roughnessMap:b,anisotropy:G,anisotropyMap:ae,clearcoat:q,clearcoatMap:Ie,clearcoatNormalMap:re,clearcoatRoughnessMap:ge,dispersion:Y,iridescence:K,iridescenceMap:He,iridescenceThicknessMap:_e,sheen:xe,sheenColorMap:de,sheenRoughnessMap:De,specularMap:ze,specularColorMap:_t,specularIntensityMap:Le,transmission:ce,transmissionMap:L,thicknessMap:$,gradientMap:j,opaque:y.transparent===!1&&y.blending===Xi&&y.alphaToCoverage===!1,alphaMap:oe,alphaTest:le,alphaHash:je,combine:y.combine,mapUv:O&&g(y.map.channel),aoMapUv:it&&g(y.aoMap.channel),lightMapUv:Re&&g(y.lightMap.channel),bumpMapUv:Ve&&g(y.bumpMap.channel),normalMapUv:Ue&&g(y.normalMap.channel),displacementMapUv:Ge&&g(y.displacementMap.channel),emissiveMapUv:pt&&g(y.emissiveMap.channel),metalnessMapUv:A&&g(y.metalnessMap.channel),roughnessMapUv:b&&g(y.roughnessMap.channel),anisotropyMapUv:ae&&g(y.anisotropyMap.channel),clearcoatMapUv:Ie&&g(y.clearcoatMap.channel),clearcoatNormalMapUv:re&&g(y.clearcoatNormalMap.channel),clearcoatRoughnessMapUv:ge&&g(y.clearcoatRoughnessMap.channel),iridescenceMapUv:He&&g(y.iridescenceMap.channel),iridescenceThicknessMapUv:_e&&g(y.iridescenceThicknessMap.channel),sheenColorMapUv:de&&g(y.sheenColorMap.channel),sheenRoughnessMapUv:De&&g(y.sheenRoughnessMap.channel),specularMapUv:ze&&g(y.specularMap.channel),specularColorMapUv:_t&&g(y.specularColorMap.channel),specularIntensityMapUv:Le&&g(y.specularIntensityMap.channel),transmissionMapUv:L&&g(y.transmissionMap.channel),thicknessMapUv:$&&g(y.thicknessMap.channel),alphaMapUv:oe&&g(y.alphaMap.channel),vertexTangents:!!W.attributes.tangent&&(Ue||G),vertexColors:y.vertexColors,vertexAlphas:y.vertexColors===!0&&!!W.attributes.color&&W.attributes.color.itemSize===4,pointsUvs:E.isPoints===!0&&!!W.attributes.uv&&(O||oe),fog:!!N,useFog:y.fog===!0,fogExp2:!!N&&N.isFogExp2,flatShading:y.flatShading===!0,sizeAttenuation:y.sizeAttenuation===!0,logarithmicDepthBuffer:u,skinning:E.isSkinnedMesh===!0,morphTargets:W.morphAttributes.position!==void 0,morphNormals:W.morphAttributes.normal!==void 0,morphColors:W.morphAttributes.color!==void 0,morphTargetsCount:me,morphTextureStride:Ne,numDirLights:v.directional.length,numPointLights:v.point.length,numSpotLights:v.spot.length,numSpotLightMaps:v.spotLightMap.length,numRectAreaLights:v.rectArea.length,numHemiLights:v.hemi.length,numDirLightShadows:v.directionalShadowMap.length,numPointLightShadows:v.pointShadowMap.length,numSpotLightShadows:v.spotShadowMap.length,numSpotLightShadowsWithMaps:v.numSpotLightShadowsWithMaps,numLightProbes:v.numLightProbes,numClippingPlanes:s.numPlanes,numClipIntersection:s.numIntersection,dithering:y.dithering,shadowMapEnabled:r.shadowMap.enabled&&I.length>0,shadowMapType:r.shadowMap.type,toneMapping:mt,useLegacyLights:r._useLegacyLights,decodeVideoTexture:O&&y.map.isVideoTexture===!0&&Oe.getTransfer(y.map.colorSpace)===$e,premultipliedAlpha:y.premultipliedAlpha,doubleSided:y.side===yt,flipSided:y.side===ut,useDepthPacking:y.depthPacking>=0,depthPacking:y.depthPacking||0,index0AttributeName:y.index0AttributeName,extensionClipCullDistance:st&&y.extensions.clipCullDistance===!0&&i.has("WEBGL_clip_cull_distance"),extensionMultiDraw:st&&y.extensions.multiDraw===!0&&i.has("WEBGL_multi_draw"),rendererExtensionParallelShaderCompile:i.has("KHR_parallel_shader_compile"),customProgramCacheKey:y.customProgramCacheKey()};return kt.vertexUv1s=l.has(1),kt.vertexUv2s=l.has(2),kt.vertexUv3s=l.has(3),l.clear(),kt}function f(y){let v=[];if(y.shaderID?v.push(y.shaderID):(v.push(y.customVertexShaderID),v.push(y.customFragmentShaderID)),y.defines!==void 0)for(let I in y.defines)v.push(I),v.push(y.defines[I]);return y.isRawShaderMaterial===!1&&(S(v,y),_(v,y),v.push(r.outputColorSpace)),v.push(y.customProgramCacheKey),v.join()}function S(y,v){y.push(v.precision),y.push(v.outputColorSpace),y.push(v.envMapMode),y.push(v.envMapCubeUVHeight),y.push(v.mapUv),y.push(v.alphaMapUv),y.push(v.lightMapUv),y.push(v.aoMapUv),y.push(v.bumpMapUv),y.push(v.normalMapUv),y.push(v.displacementMapUv),y.push(v.emissiveMapUv),y.push(v.metalnessMapUv),y.push(v.roughnessMapUv),y.push(v.anisotropyMapUv),y.push(v.clearcoatMapUv),y.push(v.clearcoatNormalMapUv),y.push(v.clearcoatRoughnessMapUv),y.push(v.iridescenceMapUv),y.push(v.iridescenceThicknessMapUv),y.push(v.sheenColorMapUv),y.push(v.sheenRoughnessMapUv),y.push(v.specularMapUv),y.push(v.specularColorMapUv),y.push(v.specularIntensityMapUv),y.push(v.transmissionMapUv),y.push(v.thicknessMapUv),y.push(v.combine),y.push(v.fogExp2),y.push(v.sizeAttenuation),y.push(v.morphTargetsCount),y.push(v.morphAttributeCount),y.push(v.numDirLights),y.push(v.numPointLights),y.push(v.numSpotLights),y.push(v.numSpotLightMaps),y.push(v.numHemiLights),y.push(v.numRectAreaLights),y.push(v.numDirLightShadows),y.push(v.numPointLightShadows),y.push(v.numSpotLightShadows),y.push(v.numSpotLightShadowsWithMaps),y.push(v.numLightProbes),y.push(v.shadowMapType),y.push(v.toneMapping),y.push(v.numClippingPlanes),y.push(v.numClipIntersection),y.push(v.depthPacking)}function _(y,v){a.disableAll(),v.supportsVertexTextures&&a.enable(0),v.instancing&&a.enable(1),v.instancingColor&&a.enable(2),v.instancingMorph&&a.enable(3),v.matcap&&a.enable(4),v.envMap&&a.enable(5),v.normalMapObjectSpace&&a.enable(6),v.normalMapTangentSpace&&a.enable(7),v.clearcoat&&a.enable(8),v.iridescence&&a.enable(9),v.alphaTest&&a.enable(10),v.vertexColors&&a.enable(11),v.vertexAlphas&&a.enable(12),v.vertexUv1s&&a.enable(13),v.vertexUv2s&&a.enable(14),v.vertexUv3s&&a.enable(15),v.vertexTangents&&a.enable(16),v.anisotropy&&a.enable(17),v.alphaHash&&a.enable(18),v.batching&&a.enable(19),v.dispersion&&a.enable(20),y.push(a.mask),a.disableAll(),v.fog&&a.enable(0),v.useFog&&a.enable(1),v.flatShading&&a.enable(2),v.logarithmicDepthBuffer&&a.enable(3),v.skinning&&a.enable(4),v.morphTargets&&a.enable(5),v.morphNormals&&a.enable(6),v.morphColors&&a.enable(7),v.premultipliedAlpha&&a.enable(8),v.shadowMapEnabled&&a.enable(9),v.useLegacyLights&&a.enable(10),v.doubleSided&&a.enable(11),v.flipSided&&a.enable(12),v.useDepthPacking&&a.enable(13),v.dithering&&a.enable(14),v.transmission&&a.enable(15),v.sheen&&a.enable(16),v.opaque&&a.enable(17),v.pointsUvs&&a.enable(18),v.decodeVideoTexture&&a.enable(19),v.alphaToCoverage&&a.enable(20),y.push(a.mask)}function R(y){let v=x[y.type],I;if(v){let F=pi[v];I=uo.clone(F.uniforms)}else I=y.uniforms;return I}function P(y,v){let I;for(let F=0,E=d.length;F<E;F++){let N=d[F];if(N.cacheKey===v){I=N,++I.usedTimes;break}}return I===void 0&&(I=new Pp(r,v,y,o),d.push(I)),I}function w(y){if(--y.usedTimes===0){let v=d.indexOf(y);d[v]=d[d.length-1],d.pop(),y.destroy()}}function T(y){c.remove(y)}function C(){c.dispose()}return{getParameters:m,getProgramCacheKey:f,getUniforms:R,acquireProgram:P,releaseProgram:w,releaseShaderCache:T,programs:d,dispose:C}}function Dp(){let r=new WeakMap;function e(o){let s=r.get(o);return s===void 0&&(s={},r.set(o,s)),s}function t(o){r.delete(o)}function i(o,s,a){r.get(o)[s]=a}function n(){r=new WeakMap}return{get:e,remove:t,update:i,dispose:n}}function av(r,e){return r.groupOrder!==e.groupOrder?r.groupOrder-e.groupOrder:r.renderOrder!==e.renderOrder?r.renderOrder-e.renderOrder:r.material.id!==e.material.id?r.material.id-e.material.id:r.z!==e.z?r.z-e.z:r.id-e.id}function Lp(r,e){return r.groupOrder!==e.groupOrder?r.groupOrder-e.groupOrder:r.renderOrder!==e.renderOrder?r.renderOrder-e.renderOrder:r.z!==e.z?e.z-r.z:r.id-e.id}function Up(){let r=[],e=0,t=[],i=[],n=[];function o(){e=0,t.length=0,i.length=0,n.length=0}function s(u,h,p,x,g,m){let f=r[e];return f===void 0?(f={id:u.id,object:u,geometry:h,material:p,groupOrder:x,renderOrder:u.renderOrder,z:g,group:m},r[e]=f):(f.id=u.id,f.object=u,f.geometry=h,f.material=p,f.groupOrder=x,f.renderOrder=u.renderOrder,f.z=g,f.group=m),e++,f}function a(u,h,p,x,g,m){let f=s(u,h,p,x,g,m);p.transmission>0?i.push(f):p.transparent===!0?n.push(f):t.push(f)}function c(u,h,p,x,g,m){let f=s(u,h,p,x,g,m);p.transmission>0?i.unshift(f):p.transparent===!0?n.unshift(f):t.unshift(f)}function l(u,h){t.length>1&&t.sort(u||av),i.length>1&&i.sort(h||Lp),n.length>1&&n.sort(h||Lp)}function d(){for(let u=e,h=r.length;u<h;u++){let p=r[u];if(p.id===null)break;p.id=null,p.object=null,p.geometry=null,p.material=null,p.group=null}}return{opaque:t,transmissive:i,transparent:n,init:o,push:a,unshift:c,finish:d,sort:l}}function Fp(){let r=new WeakMap;function e(i,n){let o=r.get(i),s;return o===void 0?(s=new Up,r.set(i,[s])):n>=o.length?(s=new Up,o.push(s)):s=o[n],s}function t(){r=new WeakMap}return{get:e,dispose:t}}function cv(){let r={};return{get:function(e){if(r[e.id]!==void 0)return r[e.id];let t;switch(e.type){case"DirectionalLight":t={direction:new D,color:new Me};break;case"SpotLight":t={position:new D,direction:new D,color:new Me,distance:0,coneCos:0,penumbraCos:0,decay:0};break;case"PointLight":t={position:new D,color:new Me,distance:0,decay:0};break;case"HemisphereLight":t={direction:new D,skyColor:new Me,groundColor:new Me};break;case"RectAreaLight":t={color:new Me,position:new D,halfWidth:new D,halfHeight:new D};break}return r[e.id]=t,t}}}function lv(){let r={};return{get:function(e){if(r[e.id]!==void 0)return r[e.id];let t;switch(e.type){case"DirectionalLight":t={shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new be};break;case"SpotLight":t={shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new be};break;case"PointLight":t={shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new be,shadowCameraNear:1,shadowCameraFar:1e3};break}return r[e.id]=t,t}}}var dv=0;function uv(r,e){return(e.castShadow?2:0)-(r.castShadow?2:0)+(e.map?1:0)-(r.map?1:0)}function Op(r){let e=new cv,t=lv(),i={version:0,hash:{directionalLength:-1,pointLength:-1,spotLength:-1,rectAreaLength:-1,hemiLength:-1,numDirectionalShadows:-1,numPointShadows:-1,numSpotShadows:-1,numSpotMaps:-1,numLightProbes:-1},ambient:[0,0,0],probe:[],directional:[],directionalShadow:[],directionalShadowMap:[],directionalShadowMatrix:[],spot:[],spotLightMap:[],spotShadow:[],spotShadowMap:[],spotLightMatrix:[],rectArea:[],rectAreaLTC1:null,rectAreaLTC2:null,point:[],pointShadow:[],pointShadowMap:[],pointShadowMatrix:[],hemi:[],numSpotLightShadowsWithMaps:0,numLightProbes:0};for(let l=0;l<9;l++)i.probe.push(new D);let n=new D,o=new Ee,s=new Ee;function a(l,d){let u=0,h=0,p=0;for(let I=0;I<9;I++)i.probe[I].set(0,0,0);let x=0,g=0,m=0,f=0,S=0,_=0,R=0,P=0,w=0,T=0,C=0;l.sort(uv);let y=d===!0?Math.PI:1;for(let I=0,F=l.length;I<F;I++){let E=l[I],N=E.color,W=E.intensity,X=E.distance,te=E.shadow&&E.shadow.map?E.shadow.map.texture:null;if(E.isAmbientLight)u+=N.r*W*y,h+=N.g*W*y,p+=N.b*W*y;else if(E.isLightProbe){for(let V=0;V<9;V++)i.probe[V].addScaledVector(E.sh.coefficients[V],W);C++}else if(E.isDirectionalLight){let V=e.get(E);if(V.color.copy(E.color).multiplyScalar(E.intensity*y),E.castShadow){let J=E.shadow,Q=t.get(E);Q.shadowBias=J.bias,Q.shadowNormalBias=J.normalBias,Q.shadowRadius=J.radius,Q.shadowMapSize=J.mapSize,i.directionalShadow[x]=Q,i.directionalShadowMap[x]=te,i.directionalShadowMatrix[x]=E.shadow.matrix,_++}i.directional[x]=V,x++}else if(E.isSpotLight){let V=e.get(E);V.position.setFromMatrixPosition(E.matrixWorld),V.color.copy(N).multiplyScalar(W*y),V.distance=X,V.coneCos=Math.cos(E.angle),V.penumbraCos=Math.cos(E.angle*(1-E.penumbra)),V.decay=E.decay,i.spot[m]=V;let J=E.shadow;if(E.map&&(i.spotLightMap[w]=E.map,w++,J.updateMatrices(E),E.castShadow&&T++),i.spotLightMatrix[m]=J.matrix,E.castShadow){let Q=t.get(E);Q.shadowBias=J.bias,Q.shadowNormalBias=J.normalBias,Q.shadowRadius=J.radius,Q.shadowMapSize=J.mapSize,i.spotShadow[m]=Q,i.spotShadowMap[m]=te,P++}m++}else if(E.isRectAreaLight){let V=e.get(E);V.color.copy(N).multiplyScalar(W),V.halfWidth.set(E.width*.5,0,0),V.halfHeight.set(0,E.height*.5,0),i.rectArea[f]=V,f++}else if(E.isPointLight){let V=e.get(E);if(V.color.copy(E.color).multiplyScalar(E.intensity*y),V.distance=E.distance,V.decay=E.decay,E.castShadow){let J=E.shadow,Q=t.get(E);Q.shadowBias=J.bias,Q.shadowNormalBias=J.normalBias,Q.shadowRadius=J.radius,Q.shadowMapSize=J.mapSize,Q.shadowCameraNear=J.camera.near,Q.shadowCameraFar=J.camera.far,i.pointShadow[g]=Q,i.pointShadowMap[g]=te,i.pointShadowMatrix[g]=E.shadow.matrix,R++}i.point[g]=V,g++}else if(E.isHemisphereLight){let V=e.get(E);V.skyColor.copy(E.color).multiplyScalar(W*y),V.groundColor.copy(E.groundColor).multiplyScalar(W*y),i.hemi[S]=V,S++}}f>0&&(r.has("OES_texture_float_linear")===!0?(i.rectAreaLTC1=ie.LTC_FLOAT_1,i.rectAreaLTC2=ie.LTC_FLOAT_2):(i.rectAreaLTC1=ie.LTC_HALF_1,i.rectAreaLTC2=ie.LTC_HALF_2)),i.ambient[0]=u,i.ambient[1]=h,i.ambient[2]=p;let v=i.hash;(v.directionalLength!==x||v.pointLength!==g||v.spotLength!==m||v.rectAreaLength!==f||v.hemiLength!==S||v.numDirectionalShadows!==_||v.numPointShadows!==R||v.numSpotShadows!==P||v.numSpotMaps!==w||v.numLightProbes!==C)&&(i.directional.length=x,i.spot.length=m,i.rectArea.length=f,i.point.length=g,i.hemi.length=S,i.directionalShadow.length=_,i.directionalShadowMap.length=_,i.pointShadow.length=R,i.pointShadowMap.length=R,i.spotShadow.length=P,i.spotShadowMap.length=P,i.directionalShadowMatrix.length=_,i.pointShadowMatrix.length=R,i.spotLightMatrix.length=P+w-T,i.spotLightMap.length=w,i.numSpotLightShadowsWithMaps=T,i.numLightProbes=C,v.directionalLength=x,v.pointLength=g,v.spotLength=m,v.rectAreaLength=f,v.hemiLength=S,v.numDirectionalShadows=_,v.numPointShadows=R,v.numSpotShadows=P,v.numSpotMaps=w,v.numLightProbes=C,i.version=dv++)}function c(l,d){let u=0,h=0,p=0,x=0,g=0,m=d.matrixWorldInverse;for(let f=0,S=l.length;f<S;f++){let _=l[f];if(_.isDirectionalLight){let R=i.directional[u];R.direction.setFromMatrixPosition(_.matrixWorld),n.setFromMatrixPosition(_.target.matrixWorld),R.direction.sub(n),R.direction.transformDirection(m),u++}else if(_.isSpotLight){let R=i.spot[p];R.position.setFromMatrixPosition(_.matrixWorld),R.position.applyMatrix4(m),R.direction.setFromMatrixPosition(_.matrixWorld),n.setFromMatrixPosition(_.target.matrixWorld),R.direction.sub(n),R.direction.transformDirection(m),p++}else if(_.isRectAreaLight){let R=i.rectArea[x];R.position.setFromMatrixPosition(_.matrixWorld),R.position.applyMatrix4(m),s.identity(),o.copy(_.matrixWorld),o.premultiply(m),s.extractRotation(o),R.halfWidth.set(_.width*.5,0,0),R.halfHeight.set(0,_.height*.5,0),R.halfWidth.applyMatrix4(s),R.halfHeight.applyMatrix4(s),x++}else if(_.isPointLight){let R=i.point[h];R.position.setFromMatrixPosition(_.matrixWorld),R.position.applyMatrix4(m),h++}else if(_.isHemisphereLight){let R=i.hemi[g];R.direction.setFromMatrixPosition(_.matrixWorld),R.direction.transformDirection(m),g++}}}return{setup:a,setupView:c,state:i}}function Np(r){let e=new Op(r),t=[],i=[];function n(d){l.camera=d,t.length=0,i.length=0}function o(d){t.push(d)}function s(d){i.push(d)}function a(d){e.setup(t,d)}function c(d){e.setupView(t,d)}let l={lightsArray:t,shadowsArray:i,camera:null,lights:e,transmissionRenderTarget:{}};return{init:n,state:l,setupLights:a,setupLightsView:c,pushLight:o,pushShadow:s}}function Bp(r){let e=new WeakMap;function t(n,o=0){let s=e.get(n),a;return s===void 0?(a=new Np(r),e.set(n,[a])):o>=s.length?(a=new Np(r),s.push(a)):a=s[o],a}function i(){e=new WeakMap}return{get:t,dispose:i}}var zs=class extends Bi{constructor(e){super(),this.isMeshDepthMaterial=!0,this.type="MeshDepthMaterial",this.depthPacking=Pd,this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.wireframe=!1,this.wireframeLinewidth=1,this.setValues(e)}copy(e){return super.copy(e),this.depthPacking=e.depthPacking,this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this}};var Vs=class extends Bi{constructor(e){super(),this.isMeshDistanceMaterial=!0,this.type="MeshDistanceMaterial",this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.setValues(e)}copy(e){return super.copy(e),this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this}};var kp=`
void main() {

	gl_Position = vec4( position, 1.0 );

}
`,zp=`
uniform sampler2D shadow_pass;
uniform vec2 resolution;
uniform float radius;

#include <packing>

void main() {

	const float samples = float( VSM_SAMPLES );

	float mean = 0.0;
	float squared_mean = 0.0;

	float uvStride = samples <= 1.0 ? 0.0 : 2.0 / ( samples - 1.0 );
	float uvStart = samples <= 1.0 ? 0.0 : - 1.0;
	for ( float i = 0.0; i < samples; i ++ ) {

		float uvOffset = uvStart + i * uvStride;

		#ifdef HORIZONTAL_PASS

			vec2 distribution = unpackRGBATo2Half( texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( uvOffset, 0.0 ) * radius ) / resolution ) );
			mean += distribution.x;
			squared_mean += distribution.y * distribution.y + distribution.x * distribution.x;

		#else

			float depth = unpackRGBAToDepth( texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( 0.0, uvOffset ) * radius ) / resolution ) );
			mean += depth;
			squared_mean += depth * depth;

		#endif

	}

	mean = mean / samples;
	squared_mean = squared_mean / samples;

	float std_dev = sqrt( squared_mean - mean * mean );

	gl_FragColor = pack2HalfToRGBA( vec2( mean, std_dev ) );

}
`;function Vp(r,e,t){let i=new _r,n=new be,o=new be,s=new ct,a=new zs({depthPacking:Id}),c=new Vs,l={},d=t.maxTextureSize,u={[ni]:ut,[ut]:ni,[yt]:yt},h=new ft({defines:{VSM_SAMPLES:8},uniforms:{shadow_pass:{value:null},resolution:{value:new be},radius:{value:4}},vertexShader:kp,fragmentShader:zp}),p=h.clone();p.defines.HORIZONTAL_PASS=1;let x=new Tt;x.setAttribute("position",new Kt(new Float32Array([-1,-1,.5,3,-1,.5,-1,3,.5]),3));let g=new Xe(x,h),m=this;this.enabled=!1,this.autoUpdate=!0,this.needsUpdate=!1,this.type=jo;let f=this.type;this.render=function(w,T,C){if(m.enabled===!1||m.autoUpdate===!1&&m.needsUpdate===!1||w.length===0)return;let y=r.getRenderTarget(),v=r.getActiveCubeFace(),I=r.getActiveMipmapLevel(),F=r.state;F.setBlending(di),F.buffers.color.setClear(1,1,1,1),F.buffers.depth.setTest(!0),F.setScissorTest(!1);let E=f!==xi&&this.type===xi,N=f===xi&&this.type!==xi;for(let W=0,X=w.length;W<X;W++){let te=w[W],V=te.shadow;if(V===void 0){console.warn("THREE.WebGLShadowMap:",te,"has no shadow.");continue}if(V.autoUpdate===!1&&V.needsUpdate===!1)continue;n.copy(V.mapSize);let J=V.getFrameExtents();if(n.multiply(J),o.copy(V.mapSize),(n.x>d||n.y>d)&&(n.x>d&&(o.x=Math.floor(d/J.x),n.x=o.x*J.x,V.mapSize.x=o.x),n.y>d&&(o.y=Math.floor(d/J.y),n.y=o.y*J.y,V.mapSize.y=o.y)),V.map===null||E===!0||N===!0){let me=this.type!==xi?{minFilter:nt,magFilter:nt}:{};V.map!==null&&V.map.dispose(),V.map=new Ot(n.x,n.y,me),V.map.texture.name=te.name+".shadowMap",V.camera.updateProjectionMatrix()}r.setRenderTarget(V.map),r.clear();let Q=V.getViewportCount();for(let me=0;me<Q;me++){let Ne=V.getViewport(me);s.set(o.x*Ne.x,o.y*Ne.y,o.x*Ne.z,o.y*Ne.w),F.viewport(s),V.updateMatrices(te,me),i=V.getFrustum(),R(T,C,V.camera,te,this.type)}V.isPointLightShadow!==!0&&this.type===xi&&S(V,C),V.needsUpdate=!1}f=this.type,m.needsUpdate=!1,r.setRenderTarget(y,v,I)};function S(w,T){let C=e.update(g);h.defines.VSM_SAMPLES!==w.blurSamples&&(h.defines.VSM_SAMPLES=w.blurSamples,p.defines.VSM_SAMPLES=w.blurSamples,h.needsUpdate=!0,p.needsUpdate=!0),w.mapPass===null&&(w.mapPass=new Ot(n.x,n.y)),h.uniforms.shadow_pass.value=w.map.texture,h.uniforms.resolution.value=w.mapSize,h.uniforms.radius.value=w.radius,r.setRenderTarget(w.mapPass),r.clear(),r.renderBufferDirect(T,null,C,h,g,null),p.uniforms.shadow_pass.value=w.mapPass.texture,p.uniforms.resolution.value=w.mapSize,p.uniforms.radius.value=w.radius,r.setRenderTarget(w.map),r.clear(),r.renderBufferDirect(T,null,C,p,g,null)}function _(w,T,C,y){let v=null,I=C.isPointLight===!0?w.customDistanceMaterial:w.customDepthMaterial;if(I!==void 0)v=I;else if(v=C.isPointLight===!0?c:a,r.localClippingEnabled&&T.clipShadows===!0&&Array.isArray(T.clippingPlanes)&&T.clippingPlanes.length!==0||T.displacementMap&&T.displacementScale!==0||T.alphaMap&&T.alphaTest>0||T.map&&T.alphaTest>0){let F=v.uuid,E=T.uuid,N=l[F];N===void 0&&(N={},l[F]=N);let W=N[E];W===void 0&&(W=v.clone(),N[E]=W,T.addEventListener("dispose",P)),v=W}if(v.visible=T.visible,v.wireframe=T.wireframe,y===xi?v.side=T.shadowSide!==null?T.shadowSide:T.side:v.side=T.shadowSide!==null?T.shadowSide:u[T.side],v.alphaMap=T.alphaMap,v.alphaTest=T.alphaTest,v.map=T.map,v.clipShadows=T.clipShadows,v.clippingPlanes=T.clippingPlanes,v.clipIntersection=T.clipIntersection,v.displacementMap=T.displacementMap,v.displacementScale=T.displacementScale,v.displacementBias=T.displacementBias,v.wireframeLinewidth=T.wireframeLinewidth,v.linewidth=T.linewidth,C.isPointLight===!0&&v.isMeshDistanceMaterial===!0){let F=r.properties.get(v);F.light=C}return v}function R(w,T,C,y,v){if(w.visible===!1)return;if(w.layers.test(T.layers)&&(w.isMesh||w.isLine||w.isPoints)&&(w.castShadow||w.receiveShadow&&v===xi)&&(!w.frustumCulled||i.intersectsObject(w))){w.modelViewMatrix.multiplyMatrices(C.matrixWorldInverse,w.matrixWorld);let E=e.update(w),N=w.material;if(Array.isArray(N)){let W=E.groups;for(let X=0,te=W.length;X<te;X++){let V=W[X],J=N[V.materialIndex];if(J&&J.visible){let Q=_(w,J,y,v);w.onBeforeShadow(r,w,T,C,E,Q,V),r.renderBufferDirect(C,null,E,Q,w,V),w.onAfterShadow(r,w,T,C,E,Q,V)}}}else if(N.visible){let W=_(w,N,y,v);w.onBeforeShadow(r,w,T,C,E,W,null),r.renderBufferDirect(C,null,E,W,w,null),w.onAfterShadow(r,w,T,C,E,W,null)}}let F=w.children;for(let E=0,N=F.length;E<N;E++)R(F[E],T,C,y,v)}function P(w){w.target.removeEventListener("dispose",P);for(let C in l){let y=l[C],v=w.target.uuid;v in y&&(y[v].dispose(),delete y[v])}}}function Gp(r){function e(){let L=!1,$=new ct,j=null,oe=new ct(0,0,0,0);return{setMask:function(le){j!==le&&!L&&(r.colorMask(le,le,le,le),j=le)},setLocked:function(le){L=le},setClear:function(le,je,st,mt,kt){kt===!0&&(le*=mt,je*=mt,st*=mt),$.set(le,je,st,mt),oe.equals($)===!1&&(r.clearColor(le,je,st,mt),oe.copy($))},reset:function(){L=!1,j=null,oe.set(-1,0,0,0)}}}function t(){let L=!1,$=null,j=null,oe=null;return{setTest:function(le){le?he(r.DEPTH_TEST):ne(r.DEPTH_TEST)},setMask:function(le){$!==le&&!L&&(r.depthMask(le),$=le)},setFunc:function(le){if(j!==le){switch(le){case rd:r.depthFunc(r.NEVER);break;case nd:r.depthFunc(r.ALWAYS);break;case od:r.depthFunc(r.LESS);break;case Jr:r.depthFunc(r.LEQUAL);break;case sd:r.depthFunc(r.EQUAL);break;case ad:r.depthFunc(r.GEQUAL);break;case cd:r.depthFunc(r.GREATER);break;case ld:r.depthFunc(r.NOTEQUAL);break;default:r.depthFunc(r.LEQUAL)}j=le}},setLocked:function(le){L=le},setClear:function(le){oe!==le&&(r.clearDepth(le),oe=le)},reset:function(){L=!1,$=null,j=null,oe=null}}}function i(){let L=!1,$=null,j=null,oe=null,le=null,je=null,st=null,mt=null,kt=null;return{setTest:function(Ke){L||(Ke?he(r.STENCIL_TEST):ne(r.STENCIL_TEST))},setMask:function(Ke){$!==Ke&&!L&&(r.stencilMask(Ke),$=Ke)},setFunc:function(Ke,Ai,Xt){(j!==Ke||oe!==Ai||le!==Xt)&&(r.stencilFunc(Ke,Ai,Xt),j=Ke,oe=Ai,le=Xt)},setOp:function(Ke,Ai,Xt){(je!==Ke||st!==Ai||mt!==Xt)&&(r.stencilOp(Ke,Ai,Xt),je=Ke,st=Ai,mt=Xt)},setLocked:function(Ke){L=Ke},setClear:function(Ke){kt!==Ke&&(r.clearStencil(Ke),kt=Ke)},reset:function(){L=!1,$=null,j=null,oe=null,le=null,je=null,st=null,mt=null,kt=null}}}let n=new e,o=new t,s=new i,a=new WeakMap,c=new WeakMap,l={},d={},u=new WeakMap,h=[],p=null,x=!1,g=null,m=null,f=null,S=null,_=null,R=null,P=null,w=new Me(0,0,0),T=0,C=!1,y=null,v=null,I=null,F=null,E=null,N=r.getParameter(r.MAX_COMBINED_TEXTURE_IMAGE_UNITS),W=!1,X=0,te=r.getParameter(r.VERSION);te.indexOf("WebGL")!==-1?(X=parseFloat(/^WebGL (\d)/.exec(te)[1]),W=X>=1):te.indexOf("OpenGL ES")!==-1&&(X=parseFloat(/^OpenGL ES (\d)/.exec(te)[1]),W=X>=2);let V=null,J={},Q=r.getParameter(r.SCISSOR_BOX),me=r.getParameter(r.VIEWPORT),Ne=new ct().fromArray(Q),tt=new ct().fromArray(me);function H(L,$,j,oe){let le=new Uint8Array(4),je=r.createTexture();r.bindTexture(L,je),r.texParameteri(L,r.TEXTURE_MIN_FILTER,r.NEAREST),r.texParameteri(L,r.TEXTURE_MAG_FILTER,r.NEAREST);for(let st=0;st<j;st++)L===r.TEXTURE_3D||L===r.TEXTURE_2D_ARRAY?r.texImage3D($,0,r.RGBA,1,1,oe,0,r.RGBA,r.UNSIGNED_BYTE,le):r.texImage2D($+st,0,r.RGBA,1,1,0,r.RGBA,r.UNSIGNED_BYTE,le);return je}let ee={};ee[r.TEXTURE_2D]=H(r.TEXTURE_2D,r.TEXTURE_2D,1),ee[r.TEXTURE_CUBE_MAP]=H(r.TEXTURE_CUBE_MAP,r.TEXTURE_CUBE_MAP_POSITIVE_X,6),ee[r.TEXTURE_2D_ARRAY]=H(r.TEXTURE_2D_ARRAY,r.TEXTURE_2D_ARRAY,1,1),ee[r.TEXTURE_3D]=H(r.TEXTURE_3D,r.TEXTURE_3D,1,1),n.setClear(0,0,0,1),o.setClear(1),s.setClear(0),he(r.DEPTH_TEST),o.setFunc(Jr),Ve(!1),Ue(_a),he(r.CULL_FACE),it(di);function he(L){l[L]!==!0&&(r.enable(L),l[L]=!0)}function ne(L){l[L]!==!1&&(r.disable(L),l[L]=!1)}function Be(L,$){return d[L]!==$?(r.bindFramebuffer(L,$),d[L]=$,L===r.DRAW_FRAMEBUFFER&&(d[r.FRAMEBUFFER]=$),L===r.FRAMEBUFFER&&(d[r.DRAW_FRAMEBUFFER]=$),!0):!1}function ke(L,$){let j=h,oe=!1;if(L){j=u.get($),j===void 0&&(j=[],u.set($,j));let le=L.textures;if(j.length!==le.length||j[0]!==r.COLOR_ATTACHMENT0){for(let je=0,st=le.length;je<st;je++)j[je]=r.COLOR_ATTACHMENT0+je;j.length=le.length,oe=!0}}else j[0]!==r.BACK&&(j[0]=r.BACK,oe=!0);oe&&r.drawBuffers(j)}function O(L){return p!==L?(r.useProgram(L),p=L,!0):!1}let ot={[qi]:r.FUNC_ADD,[zl]:r.FUNC_SUBTRACT,[Vl]:r.FUNC_REVERSE_SUBTRACT};ot[Gl]=r.MIN,ot[Wl]=r.MAX;let ve={[Hl]:r.ZERO,[jl]:r.ONE,[Xl]:r.SRC_COLOR,[Qn]:r.SRC_ALPHA,[Jl]:r.SRC_ALPHA_SATURATE,[Kl]:r.DST_COLOR,[$l]:r.DST_ALPHA,[ql]:r.ONE_MINUS_SRC_COLOR,[eo]:r.ONE_MINUS_SRC_ALPHA,[Zl]:r.ONE_MINUS_DST_COLOR,[Yl]:r.ONE_MINUS_DST_ALPHA,[Ql]:r.CONSTANT_COLOR,[ed]:r.ONE_MINUS_CONSTANT_COLOR,[td]:r.CONSTANT_ALPHA,[id]:r.ONE_MINUS_CONSTANT_ALPHA};function it(L,$,j,oe,le,je,st,mt,kt,Ke){if(L===di){x===!0&&(ne(r.BLEND),x=!1);return}if(x===!1&&(he(r.BLEND),x=!0),L!==kl){if(L!==g||Ke!==C){if((m!==qi||_!==qi)&&(r.blendEquation(r.FUNC_ADD),m=qi,_=qi),Ke)switch(L){case Xi:r.blendFuncSeparate(r.ONE,r.ONE_MINUS_SRC_ALPHA,r.ONE,r.ONE_MINUS_SRC_ALPHA);break;case ya:r.blendFunc(r.ONE,r.ONE);break;case Sa:r.blendFuncSeparate(r.ZERO,r.ONE_MINUS_SRC_COLOR,r.ZERO,r.ONE);break;case ba:r.blendFuncSeparate(r.ZERO,r.SRC_COLOR,r.ZERO,r.SRC_ALPHA);break;default:console.error("THREE.WebGLState: Invalid blending: ",L);break}else switch(L){case Xi:r.blendFuncSeparate(r.SRC_ALPHA,r.ONE_MINUS_SRC_ALPHA,r.ONE,r.ONE_MINUS_SRC_ALPHA);break;case ya:r.blendFunc(r.SRC_ALPHA,r.ONE);break;case Sa:r.blendFuncSeparate(r.ZERO,r.ONE_MINUS_SRC_COLOR,r.ZERO,r.ONE);break;case ba:r.blendFunc(r.ZERO,r.SRC_COLOR);break;default:console.error("THREE.WebGLState: Invalid blending: ",L);break}f=null,S=null,R=null,P=null,w.set(0,0,0),T=0,g=L,C=Ke}return}le=le||$,je=je||j,st=st||oe,($!==m||le!==_)&&(r.blendEquationSeparate(ot[$],ot[le]),m=$,_=le),(j!==f||oe!==S||je!==R||st!==P)&&(r.blendFuncSeparate(ve[j],ve[oe],ve[je],ve[st]),f=j,S=oe,R=je,P=st),(mt.equals(w)===!1||kt!==T)&&(r.blendColor(mt.r,mt.g,mt.b,kt),w.copy(mt),T=kt),g=L,C=!1}function Re(L,$){L.side===yt?ne(r.CULL_FACE):he(r.CULL_FACE);let j=L.side===ut;$&&(j=!j),Ve(j),L.blending===Xi&&L.transparent===!1?it(di):it(L.blending,L.blendEquation,L.blendSrc,L.blendDst,L.blendEquationAlpha,L.blendSrcAlpha,L.blendDstAlpha,L.blendColor,L.blendAlpha,L.premultipliedAlpha),o.setFunc(L.depthFunc),o.setTest(L.depthTest),o.setMask(L.depthWrite),n.setMask(L.colorWrite);let oe=L.stencilWrite;s.setTest(oe),oe&&(s.setMask(L.stencilWriteMask),s.setFunc(L.stencilFunc,L.stencilRef,L.stencilFuncMask),s.setOp(L.stencilFail,L.stencilZFail,L.stencilZPass)),pt(L.polygonOffset,L.polygonOffsetFactor,L.polygonOffsetUnits),L.alphaToCoverage===!0?he(r.SAMPLE_ALPHA_TO_COVERAGE):ne(r.SAMPLE_ALPHA_TO_COVERAGE)}function Ve(L){y!==L&&(L?r.frontFace(r.CW):r.frontFace(r.CCW),y=L)}function Ue(L){L!==Nl?(he(r.CULL_FACE),L!==v&&(L===_a?r.cullFace(r.BACK):L===Bl?r.cullFace(r.FRONT):r.cullFace(r.FRONT_AND_BACK))):ne(r.CULL_FACE),v=L}function Ge(L){L!==I&&(W&&r.lineWidth(L),I=L)}function pt(L,$,j){L?(he(r.POLYGON_OFFSET_FILL),(F!==$||E!==j)&&(r.polygonOffset($,j),F=$,E=j)):ne(r.POLYGON_OFFSET_FILL)}function A(L){L?he(r.SCISSOR_TEST):ne(r.SCISSOR_TEST)}function b(L){L===void 0&&(L=r.TEXTURE0+N-1),V!==L&&(r.activeTexture(L),V=L)}function G(L,$,j){j===void 0&&(V===null?j=r.TEXTURE0+N-1:j=V);let oe=J[j];oe===void 0&&(oe={type:void 0,texture:void 0},J[j]=oe),(oe.type!==L||oe.texture!==$)&&(V!==j&&(r.activeTexture(j),V=j),r.bindTexture(L,$||ee[L]),oe.type=L,oe.texture=$)}function q(){let L=J[V];L!==void 0&&L.type!==void 0&&(r.bindTexture(L.type,null),L.type=void 0,L.texture=void 0)}function Y(){try{r.compressedTexImage2D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function K(){try{r.compressedTexImage3D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function xe(){try{r.texSubImage2D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function ce(){try{r.texSubImage3D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function ae(){try{r.compressedTexSubImage2D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function Ie(){try{r.compressedTexSubImage3D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function re(){try{r.texStorage2D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function ge(){try{r.texStorage3D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function He(){try{r.texImage2D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function _e(){try{r.texImage3D.apply(r,arguments)}catch(L){console.error("THREE.WebGLState:",L)}}function de(L){Ne.equals(L)===!1&&(r.scissor(L.x,L.y,L.z,L.w),Ne.copy(L))}function De(L){tt.equals(L)===!1&&(r.viewport(L.x,L.y,L.z,L.w),tt.copy(L))}function ze(L,$){let j=c.get($);j===void 0&&(j=new WeakMap,c.set($,j));let oe=j.get(L);oe===void 0&&(oe=r.getUniformBlockIndex($,L.name),j.set(L,oe))}function _t(L,$){let oe=c.get($).get(L);a.get($)!==oe&&(r.uniformBlockBinding($,oe,L.__bindingPointIndex),a.set($,oe))}function Le(){r.disable(r.BLEND),r.disable(r.CULL_FACE),r.disable(r.DEPTH_TEST),r.disable(r.POLYGON_OFFSET_FILL),r.disable(r.SCISSOR_TEST),r.disable(r.STENCIL_TEST),r.disable(r.SAMPLE_ALPHA_TO_COVERAGE),r.blendEquation(r.FUNC_ADD),r.blendFunc(r.ONE,r.ZERO),r.blendFuncSeparate(r.ONE,r.ZERO,r.ONE,r.ZERO),r.blendColor(0,0,0,0),r.colorMask(!0,!0,!0,!0),r.clearColor(0,0,0,0),r.depthMask(!0),r.depthFunc(r.LESS),r.clearDepth(1),r.stencilMask(4294967295),r.stencilFunc(r.ALWAYS,0,4294967295),r.stencilOp(r.KEEP,r.KEEP,r.KEEP),r.clearStencil(0),r.cullFace(r.BACK),r.frontFace(r.CCW),r.polygonOffset(0,0),r.activeTexture(r.TEXTURE0),r.bindFramebuffer(r.FRAMEBUFFER,null),r.bindFramebuffer(r.DRAW_FRAMEBUFFER,null),r.bindFramebuffer(r.READ_FRAMEBUFFER,null),r.useProgram(null),r.lineWidth(1),r.scissor(0,0,r.canvas.width,r.canvas.height),r.viewport(0,0,r.canvas.width,r.canvas.height),l={},V=null,J={},d={},u=new WeakMap,h=[],p=null,x=!1,g=null,m=null,f=null,S=null,_=null,R=null,P=null,w=new Me(0,0,0),T=0,C=!1,y=null,v=null,I=null,F=null,E=null,Ne.set(0,0,r.canvas.width,r.canvas.height),tt.set(0,0,r.canvas.width,r.canvas.height),n.reset(),o.reset(),s.reset()}return{buffers:{color:n,depth:o,stencil:s},enable:he,disable:ne,bindFramebuffer:Be,drawBuffers:ke,useProgram:O,setBlending:it,setMaterial:Re,setFlipSided:Ve,setCullFace:Ue,setLineWidth:Ge,setPolygonOffset:pt,setScissorTest:A,activeTexture:b,bindTexture:G,unbindTexture:q,compressedTexImage2D:Y,compressedTexImage3D:K,texImage2D:He,texImage3D:_e,updateUBOMapping:ze,uniformBlockBinding:_t,texStorage2D:re,texStorage3D:ge,texSubImage2D:xe,texSubImage3D:ce,compressedTexSubImage2D:ae,compressedTexSubImage3D:Ie,scissor:de,viewport:De,reset:Le}}function Wp(r,e,t,i,n,o,s){let a=e.has("WEBGL_multisampled_render_to_texture")?e.get("WEBGL_multisampled_render_to_texture"):null,c=typeof navigator>"u"?!1:/OculusBrowser/g.test(navigator.userAgent),l=new be,d=new WeakMap,u,h=new WeakMap,p=!1;try{p=typeof OffscreenCanvas<"u"&&new OffscreenCanvas(1,1).getContext("2d")!==null}catch{}function x(A,b){return p?new OffscreenCanvas(A,b):Gd("canvas")}function g(A,b,G){let q=1,Y=pt(A);if((Y.width>G||Y.height>G)&&(q=G/Math.max(Y.width,Y.height)),q<1)if(typeof VideoFrame<"u"&&A instanceof VideoFrame){let K=Math.floor(q*Y.width),xe=Math.floor(q*Y.height);u===void 0&&(u=x(K,xe));let ce=b?x(K,xe):u;return ce.width=K,ce.height=xe,ce.getContext("2d").drawImage(A,0,0,K,xe),console.warn("THREE.WebGLRenderer: Texture has been resized from ("+Y.width+"x"+Y.height+") to ("+K+"x"+xe+")."),ce}else return"data"in A&&console.warn("THREE.WebGLRenderer: Image in DataTexture is too big ("+Y.width+"x"+Y.height+")."),A;return A}function m(A){return A.generateMipmaps&&A.minFilter!==nt&&A.minFilter!==ht}function f(A){r.generateMipmap(A)}function S(A,b,G,q,Y=!1){if(A!==null){if(r[A]!==void 0)return r[A];console.warn("THREE.WebGLRenderer: Attempt to use non-existing WebGL internal format '"+A+"'")}let K=b;if(b===r.RED&&(G===r.FLOAT&&(K=r.R32F),G===r.HALF_FLOAT&&(K=r.R16F),G===r.UNSIGNED_BYTE&&(K=r.R8)),b===r.RED_INTEGER&&(G===r.UNSIGNED_BYTE&&(K=r.R8UI),G===r.UNSIGNED_SHORT&&(K=r.R16UI),G===r.UNSIGNED_INT&&(K=r.R32UI),G===r.BYTE&&(K=r.R8I),G===r.SHORT&&(K=r.R16I),G===r.INT&&(K=r.R32I)),b===r.RG&&(G===r.FLOAT&&(K=r.RG32F),G===r.HALF_FLOAT&&(K=r.RG16F),G===r.UNSIGNED_BYTE&&(K=r.RG8)),b===r.RG_INTEGER&&(G===r.UNSIGNED_BYTE&&(K=r.RG8UI),G===r.UNSIGNED_SHORT&&(K=r.RG16UI),G===r.UNSIGNED_INT&&(K=r.RG32UI),G===r.BYTE&&(K=r.RG8I),G===r.SHORT&&(K=r.RG16I),G===r.INT&&(K=r.RG32I)),b===r.RGB&&G===r.UNSIGNED_INT_5_9_9_9_REV&&(K=r.RGB9_E5),b===r.RGBA){let xe=Y?en:Oe.getTransfer(q);G===r.FLOAT&&(K=r.RGBA32F),G===r.HALF_FLOAT&&(K=r.RGBA16F),G===r.UNSIGNED_BYTE&&(K=xe===$e?r.SRGB8_ALPHA8:r.RGBA8),G===r.UNSIGNED_SHORT_4_4_4_4&&(K=r.RGBA4),G===r.UNSIGNED_SHORT_5_5_5_1&&(K=r.RGB5_A1)}return(K===r.R16F||K===r.R32F||K===r.RG16F||K===r.RG32F||K===r.RGBA16F||K===r.RGBA32F)&&e.get("EXT_color_buffer_float"),K}function _(A,b){return m(A)===!0||A.isFramebufferTexture&&A.minFilter!==nt&&A.minFilter!==ht?Math.log2(Math.max(b.width,b.height))+1:A.mipmaps!==void 0&&A.mipmaps.length>0?A.mipmaps.length:A.isCompressedTexture&&Array.isArray(A.image)?b.mipmaps.length:1}function R(A){let b=A.target;b.removeEventListener("dispose",R),w(b),b.isVideoTexture&&d.delete(b)}function P(A){let b=A.target;b.removeEventListener("dispose",P),C(b)}function w(A){let b=i.get(A);if(b.__webglInit===void 0)return;let G=A.source,q=h.get(G);if(q){let Y=q[b.__cacheKey];Y.usedTimes--,Y.usedTimes===0&&T(A),Object.keys(q).length===0&&h.delete(G)}i.remove(A)}function T(A){let b=i.get(A);r.deleteTexture(b.__webglTexture);let G=A.source,q=h.get(G);delete q[b.__cacheKey],s.memory.textures--}function C(A){let b=i.get(A);if(A.depthTexture&&A.depthTexture.dispose(),A.isWebGLCubeRenderTarget)for(let q=0;q<6;q++){if(Array.isArray(b.__webglFramebuffer[q]))for(let Y=0;Y<b.__webglFramebuffer[q].length;Y++)r.deleteFramebuffer(b.__webglFramebuffer[q][Y]);else r.deleteFramebuffer(b.__webglFramebuffer[q]);b.__webglDepthbuffer&&r.deleteRenderbuffer(b.__webglDepthbuffer[q])}else{if(Array.isArray(b.__webglFramebuffer))for(let q=0;q<b.__webglFramebuffer.length;q++)r.deleteFramebuffer(b.__webglFramebuffer[q]);else r.deleteFramebuffer(b.__webglFramebuffer);if(b.__webglDepthbuffer&&r.deleteRenderbuffer(b.__webglDepthbuffer),b.__webglMultisampledFramebuffer&&r.deleteFramebuffer(b.__webglMultisampledFramebuffer),b.__webglColorRenderbuffer)for(let q=0;q<b.__webglColorRenderbuffer.length;q++)b.__webglColorRenderbuffer[q]&&r.deleteRenderbuffer(b.__webglColorRenderbuffer[q]);b.__webglDepthRenderbuffer&&r.deleteRenderbuffer(b.__webglDepthRenderbuffer)}let G=A.textures;for(let q=0,Y=G.length;q<Y;q++){let K=i.get(G[q]);K.__webglTexture&&(r.deleteTexture(K.__webglTexture),s.memory.textures--),i.remove(G[q])}i.remove(A)}let y=0;function v(){y=0}function I(){let A=y;return A>=n.maxTextures&&console.warn("THREE.WebGLTextures: Trying to use "+A+" texture units while this GPU supports only "+n.maxTextures),y+=1,A}function F(A){let b=[];return b.push(A.wrapS),b.push(A.wrapT),b.push(A.wrapR||0),b.push(A.magFilter),b.push(A.minFilter),b.push(A.anisotropy),b.push(A.internalFormat),b.push(A.format),b.push(A.type),b.push(A.generateMipmaps),b.push(A.premultiplyAlpha),b.push(A.flipY),b.push(A.unpackAlignment),b.push(A.colorSpace),b.join()}function E(A,b){let G=i.get(A);if(A.isVideoTexture&&Ue(A),A.isRenderTargetTexture===!1&&A.version>0&&G.__version!==A.version){let q=A.image;if(q===null)console.warn("THREE.WebGLRenderer: Texture marked for update but no image data found.");else if(q.complete===!1)console.warn("THREE.WebGLRenderer: Texture marked for update but image is incomplete");else{Ne(G,A,b);return}}t.bindTexture(r.TEXTURE_2D,G.__webglTexture,r.TEXTURE0+b)}function N(A,b){let G=i.get(A);if(A.version>0&&G.__version!==A.version){Ne(G,A,b);return}t.bindTexture(r.TEXTURE_2D_ARRAY,G.__webglTexture,r.TEXTURE0+b)}function W(A,b){let G=i.get(A);if(A.version>0&&G.__version!==A.version){Ne(G,A,b);return}t.bindTexture(r.TEXTURE_3D,G.__webglTexture,r.TEXTURE0+b)}function X(A,b){let G=i.get(A);if(A.version>0&&G.__version!==A.version){tt(G,A,b);return}t.bindTexture(r.TEXTURE_CUBE_MAP,G.__webglTexture,r.TEXTURE0+b)}let te={[ro]:r.REPEAT,[Yt]:r.CLAMP_TO_EDGE,[no]:r.MIRRORED_REPEAT},V={[nt]:r.NEAREST,[_d]:r.NEAREST_MIPMAP_NEAREST,[oo]:r.NEAREST_MIPMAP_LINEAR,[ht]:r.LINEAR,[qo]:r.LINEAR_MIPMAP_NEAREST,[_i]:r.LINEAR_MIPMAP_LINEAR},J={[Ud]:r.NEVER,[zd]:r.ALWAYS,[Fd]:r.LESS,[ss]:r.LEQUAL,[Od]:r.EQUAL,[kd]:r.GEQUAL,[Nd]:r.GREATER,[Bd]:r.NOTEQUAL};function Q(A,b){if(b.type===oi&&e.has("OES_texture_float_linear")===!1&&(b.magFilter===ht||b.magFilter===qo||b.magFilter===oo||b.magFilter===_i||b.minFilter===ht||b.minFilter===qo||b.minFilter===oo||b.minFilter===_i)&&console.warn("THREE.WebGLRenderer: Unable to use linear filtering with floating point textures. OES_texture_float_linear not supported on this device."),r.texParameteri(A,r.TEXTURE_WRAP_S,te[b.wrapS]),r.texParameteri(A,r.TEXTURE_WRAP_T,te[b.wrapT]),(A===r.TEXTURE_3D||A===r.TEXTURE_2D_ARRAY)&&r.texParameteri(A,r.TEXTURE_WRAP_R,te[b.wrapR]),r.texParameteri(A,r.TEXTURE_MAG_FILTER,V[b.magFilter]),r.texParameteri(A,r.TEXTURE_MIN_FILTER,V[b.minFilter]),b.compareFunction&&(r.texParameteri(A,r.TEXTURE_COMPARE_MODE,r.COMPARE_REF_TO_TEXTURE),r.texParameteri(A,r.TEXTURE_COMPARE_FUNC,J[b.compareFunction])),e.has("EXT_texture_filter_anisotropic")===!0){if(b.magFilter===nt||b.minFilter!==oo&&b.minFilter!==_i||b.type===oi&&e.has("OES_texture_float_linear")===!1)return;if(b.anisotropy>1||i.get(b).__currentAnisotropy){let G=e.get("EXT_texture_filter_anisotropic");r.texParameterf(A,G.TEXTURE_MAX_ANISOTROPY_EXT,Math.min(b.anisotropy,n.getMaxAnisotropy())),i.get(b).__currentAnisotropy=b.anisotropy}}}function me(A,b){let G=!1;A.__webglInit===void 0&&(A.__webglInit=!0,b.addEventListener("dispose",R));let q=b.source,Y=h.get(q);Y===void 0&&(Y={},h.set(q,Y));let K=F(b);if(K!==A.__cacheKey){Y[K]===void 0&&(Y[K]={texture:r.createTexture(),usedTimes:0},s.memory.textures++,G=!0),Y[K].usedTimes++;let xe=Y[A.__cacheKey];xe!==void 0&&(Y[A.__cacheKey].usedTimes--,xe.usedTimes===0&&T(b)),A.__cacheKey=K,A.__webglTexture=Y[K].texture}return G}function Ne(A,b,G){let q=r.TEXTURE_2D;(b.isDataArrayTexture||b.isCompressedArrayTexture)&&(q=r.TEXTURE_2D_ARRAY),b.isData3DTexture&&(q=r.TEXTURE_3D);let Y=me(A,b),K=b.source;t.bindTexture(q,A.__webglTexture,r.TEXTURE0+G);let xe=i.get(K);if(K.version!==xe.__version||Y===!0){t.activeTexture(r.TEXTURE0+G);let ce=Oe.getPrimaries(Oe.workingColorSpace),ae=b.colorSpace===hi?null:Oe.getPrimaries(b.colorSpace),Ie=b.colorSpace===hi||ce===ae?r.NONE:r.BROWSER_DEFAULT_WEBGL;r.pixelStorei(r.UNPACK_FLIP_Y_WEBGL,b.flipY),r.pixelStorei(r.UNPACK_PREMULTIPLY_ALPHA_WEBGL,b.premultiplyAlpha),r.pixelStorei(r.UNPACK_ALIGNMENT,b.unpackAlignment),r.pixelStorei(r.UNPACK_COLORSPACE_CONVERSION_WEBGL,Ie);let re=g(b.image,!1,n.maxTextureSize);re=Ge(b,re);let ge=o.convert(b.format,b.colorSpace),He=o.convert(b.type),_e=S(b.internalFormat,ge,He,b.colorSpace,b.isVideoTexture);Q(q,b);let de,De=b.mipmaps,ze=b.isVideoTexture!==!0,_t=xe.__version===void 0||Y===!0,Le=K.dataReady,L=_(b,re);if(b.isDepthTexture)_e=r.DEPTH_COMPONENT16,b.type===oi?_e=r.DEPTH_COMPONENT32F:b.type===yi?_e=r.DEPTH_COMPONENT24:b.type===Fi&&(_e=r.DEPTH24_STENCIL8),_t&&(ze?t.texStorage2D(r.TEXTURE_2D,1,_e,re.width,re.height):t.texImage2D(r.TEXTURE_2D,0,_e,re.width,re.height,0,ge,He,null));else if(b.isDataTexture)if(De.length>0){ze&&_t&&t.texStorage2D(r.TEXTURE_2D,L,_e,De[0].width,De[0].height);for(let $=0,j=De.length;$<j;$++)de=De[$],ze?Le&&t.texSubImage2D(r.TEXTURE_2D,$,0,0,de.width,de.height,ge,He,de.data):t.texImage2D(r.TEXTURE_2D,$,_e,de.width,de.height,0,ge,He,de.data);b.generateMipmaps=!1}else ze?(_t&&t.texStorage2D(r.TEXTURE_2D,L,_e,re.width,re.height),Le&&t.texSubImage2D(r.TEXTURE_2D,0,0,0,re.width,re.height,ge,He,re.data)):t.texImage2D(r.TEXTURE_2D,0,_e,re.width,re.height,0,ge,He,re.data);else if(b.isCompressedTexture)if(b.isCompressedArrayTexture){ze&&_t&&t.texStorage3D(r.TEXTURE_2D_ARRAY,L,_e,De[0].width,De[0].height,re.depth);for(let $=0,j=De.length;$<j;$++)de=De[$],b.format!==Mt?ge!==null?ze?Le&&t.compressedTexSubImage3D(r.TEXTURE_2D_ARRAY,$,0,0,0,de.width,de.height,re.depth,ge,de.data,0,0):t.compressedTexImage3D(r.TEXTURE_2D_ARRAY,$,_e,de.width,de.height,re.depth,0,de.data,0,0):console.warn("THREE.WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()"):ze?Le&&t.texSubImage3D(r.TEXTURE_2D_ARRAY,$,0,0,0,de.width,de.height,re.depth,ge,He,de.data):t.texImage3D(r.TEXTURE_2D_ARRAY,$,_e,de.width,de.height,re.depth,0,ge,He,de.data)}else{ze&&_t&&t.texStorage2D(r.TEXTURE_2D,L,_e,De[0].width,De[0].height);for(let $=0,j=De.length;$<j;$++)de=De[$],b.format!==Mt?ge!==null?ze?Le&&t.compressedTexSubImage2D(r.TEXTURE_2D,$,0,0,de.width,de.height,ge,de.data):t.compressedTexImage2D(r.TEXTURE_2D,$,_e,de.width,de.height,0,de.data):console.warn("THREE.WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()"):ze?Le&&t.texSubImage2D(r.TEXTURE_2D,$,0,0,de.width,de.height,ge,He,de.data):t.texImage2D(r.TEXTURE_2D,$,_e,de.width,de.height,0,ge,He,de.data)}else if(b.isDataArrayTexture)ze?(_t&&t.texStorage3D(r.TEXTURE_2D_ARRAY,L,_e,re.width,re.height,re.depth),Le&&t.texSubImage3D(r.TEXTURE_2D_ARRAY,0,0,0,0,re.width,re.height,re.depth,ge,He,re.data)):t.texImage3D(r.TEXTURE_2D_ARRAY,0,_e,re.width,re.height,re.depth,0,ge,He,re.data);else if(b.isData3DTexture)ze?(_t&&t.texStorage3D(r.TEXTURE_3D,L,_e,re.width,re.height,re.depth),Le&&t.texSubImage3D(r.TEXTURE_3D,0,0,0,0,re.width,re.height,re.depth,ge,He,re.data)):t.texImage3D(r.TEXTURE_3D,0,_e,re.width,re.height,re.depth,0,ge,He,re.data);else if(b.isFramebufferTexture){if(_t)if(ze)t.texStorage2D(r.TEXTURE_2D,L,_e,re.width,re.height);else{let $=re.width,j=re.height;for(let oe=0;oe<L;oe++)t.texImage2D(r.TEXTURE_2D,oe,_e,$,j,0,ge,He,null),$>>=1,j>>=1}}else if(De.length>0){if(ze&&_t){let $=pt(De[0]);t.texStorage2D(r.TEXTURE_2D,L,_e,$.width,$.height)}for(let $=0,j=De.length;$<j;$++)de=De[$],ze?Le&&t.texSubImage2D(r.TEXTURE_2D,$,0,0,ge,He,de):t.texImage2D(r.TEXTURE_2D,$,_e,ge,He,de);b.generateMipmaps=!1}else if(ze){if(_t){let $=pt(re);t.texStorage2D(r.TEXTURE_2D,L,_e,$.width,$.height)}Le&&t.texSubImage2D(r.TEXTURE_2D,0,0,0,ge,He,re)}else t.texImage2D(r.TEXTURE_2D,0,_e,ge,He,re);m(b)&&f(q),xe.__version=K.version,b.onUpdate&&b.onUpdate(b)}A.__version=b.version}function tt(A,b,G){if(b.image.length!==6)return;let q=me(A,b),Y=b.source;t.bindTexture(r.TEXTURE_CUBE_MAP,A.__webglTexture,r.TEXTURE0+G);let K=i.get(Y);if(Y.version!==K.__version||q===!0){t.activeTexture(r.TEXTURE0+G);let xe=Oe.getPrimaries(Oe.workingColorSpace),ce=b.colorSpace===hi?null:Oe.getPrimaries(b.colorSpace),ae=b.colorSpace===hi||xe===ce?r.NONE:r.BROWSER_DEFAULT_WEBGL;r.pixelStorei(r.UNPACK_FLIP_Y_WEBGL,b.flipY),r.pixelStorei(r.UNPACK_PREMULTIPLY_ALPHA_WEBGL,b.premultiplyAlpha),r.pixelStorei(r.UNPACK_ALIGNMENT,b.unpackAlignment),r.pixelStorei(r.UNPACK_COLORSPACE_CONVERSION_WEBGL,ae);let Ie=b.isCompressedTexture||b.image[0].isCompressedTexture,re=b.image[0]&&b.image[0].isDataTexture,ge=[];for(let j=0;j<6;j++)!Ie&&!re?ge[j]=g(b.image[j],!0,n.maxCubemapSize):ge[j]=re?b.image[j].image:b.image[j],ge[j]=Ge(b,ge[j]);let He=ge[0],_e=o.convert(b.format,b.colorSpace),de=o.convert(b.type),De=S(b.internalFormat,_e,de,b.colorSpace),ze=b.isVideoTexture!==!0,_t=K.__version===void 0||q===!0,Le=Y.dataReady,L=_(b,He);Q(r.TEXTURE_CUBE_MAP,b);let $;if(Ie){ze&&_t&&t.texStorage2D(r.TEXTURE_CUBE_MAP,L,De,He.width,He.height);for(let j=0;j<6;j++){$=ge[j].mipmaps;for(let oe=0;oe<$.length;oe++){let le=$[oe];b.format!==Mt?_e!==null?ze?Le&&t.compressedTexSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe,0,0,le.width,le.height,_e,le.data):t.compressedTexImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe,De,le.width,le.height,0,le.data):console.warn("THREE.WebGLRenderer: Attempt to load unsupported compressed texture format in .setTextureCube()"):ze?Le&&t.texSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe,0,0,le.width,le.height,_e,de,le.data):t.texImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe,De,le.width,le.height,0,_e,de,le.data)}}}else{if($=b.mipmaps,ze&&_t){$.length>0&&L++;let j=pt(ge[0]);t.texStorage2D(r.TEXTURE_CUBE_MAP,L,De,j.width,j.height)}for(let j=0;j<6;j++)if(re){ze?Le&&t.texSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,0,0,0,ge[j].width,ge[j].height,_e,de,ge[j].data):t.texImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,0,De,ge[j].width,ge[j].height,0,_e,de,ge[j].data);for(let oe=0;oe<$.length;oe++){let je=$[oe].image[j].image;ze?Le&&t.texSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe+1,0,0,je.width,je.height,_e,de,je.data):t.texImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe+1,De,je.width,je.height,0,_e,de,je.data)}}else{ze?Le&&t.texSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,0,0,0,_e,de,ge[j]):t.texImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,0,De,_e,de,ge[j]);for(let oe=0;oe<$.length;oe++){let le=$[oe];ze?Le&&t.texSubImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe+1,0,0,_e,de,le.image[j]):t.texImage2D(r.TEXTURE_CUBE_MAP_POSITIVE_X+j,oe+1,De,_e,de,le.image[j])}}}m(b)&&f(r.TEXTURE_CUBE_MAP),K.__version=Y.version,b.onUpdate&&b.onUpdate(b)}A.__version=b.version}function H(A,b,G,q,Y,K){let xe=o.convert(G.format,G.colorSpace),ce=o.convert(G.type),ae=S(G.internalFormat,xe,ce,G.colorSpace);if(!i.get(b).__hasExternalTextures){let re=Math.max(1,b.width>>K),ge=Math.max(1,b.height>>K);Y===r.TEXTURE_3D||Y===r.TEXTURE_2D_ARRAY?t.texImage3D(Y,K,ae,re,ge,b.depth,0,xe,ce,null):t.texImage2D(Y,K,ae,re,ge,0,xe,ce,null)}t.bindFramebuffer(r.FRAMEBUFFER,A),Ve(b)?a.framebufferTexture2DMultisampleEXT(r.FRAMEBUFFER,q,Y,i.get(G).__webglTexture,0,Re(b)):(Y===r.TEXTURE_2D||Y>=r.TEXTURE_CUBE_MAP_POSITIVE_X&&Y<=r.TEXTURE_CUBE_MAP_NEGATIVE_Z)&&r.framebufferTexture2D(r.FRAMEBUFFER,q,Y,i.get(G).__webglTexture,K),t.bindFramebuffer(r.FRAMEBUFFER,null)}function ee(A,b,G){if(r.bindRenderbuffer(r.RENDERBUFFER,A),b.depthBuffer&&!b.stencilBuffer){let q=r.DEPTH_COMPONENT24;if(G||Ve(b)){let Y=b.depthTexture;Y&&Y.isDepthTexture&&(Y.type===oi?q=r.DEPTH_COMPONENT32F:Y.type===yi&&(q=r.DEPTH_COMPONENT24));let K=Re(b);Ve(b)?a.renderbufferStorageMultisampleEXT(r.RENDERBUFFER,K,q,b.width,b.height):r.renderbufferStorageMultisample(r.RENDERBUFFER,K,q,b.width,b.height)}else r.renderbufferStorage(r.RENDERBUFFER,q,b.width,b.height);r.framebufferRenderbuffer(r.FRAMEBUFFER,r.DEPTH_ATTACHMENT,r.RENDERBUFFER,A)}else if(b.depthBuffer&&b.stencilBuffer){let q=Re(b);G&&Ve(b)===!1?r.renderbufferStorageMultisample(r.RENDERBUFFER,q,r.DEPTH24_STENCIL8,b.width,b.height):Ve(b)?a.renderbufferStorageMultisampleEXT(r.RENDERBUFFER,q,r.DEPTH24_STENCIL8,b.width,b.height):r.renderbufferStorage(r.RENDERBUFFER,r.DEPTH_STENCIL,b.width,b.height),r.framebufferRenderbuffer(r.FRAMEBUFFER,r.DEPTH_STENCIL_ATTACHMENT,r.RENDERBUFFER,A)}else{let q=b.textures;for(let Y=0;Y<q.length;Y++){let K=q[Y],xe=o.convert(K.format,K.colorSpace),ce=o.convert(K.type),ae=S(K.internalFormat,xe,ce,K.colorSpace),Ie=Re(b);G&&Ve(b)===!1?r.renderbufferStorageMultisample(r.RENDERBUFFER,Ie,ae,b.width,b.height):Ve(b)?a.renderbufferStorageMultisampleEXT(r.RENDERBUFFER,Ie,ae,b.width,b.height):r.renderbufferStorage(r.RENDERBUFFER,ae,b.width,b.height)}}r.bindRenderbuffer(r.RENDERBUFFER,null)}function he(A,b){if(b&&b.isWebGLCubeRenderTarget)throw new Error("Depth Texture with cube render targets is not supported");if(t.bindFramebuffer(r.FRAMEBUFFER,A),!(b.depthTexture&&b.depthTexture.isDepthTexture))throw new Error("renderTarget.depthTexture must be an instance of THREE.DepthTexture");(!i.get(b.depthTexture).__webglTexture||b.depthTexture.image.width!==b.width||b.depthTexture.image.height!==b.height)&&(b.depthTexture.image.width=b.width,b.depthTexture.image.height=b.height,b.depthTexture.needsUpdate=!0),E(b.depthTexture,0);let q=i.get(b.depthTexture).__webglTexture,Y=Re(b);if(b.depthTexture.format===Oi)Ve(b)?a.framebufferTexture2DMultisampleEXT(r.FRAMEBUFFER,r.DEPTH_ATTACHMENT,r.TEXTURE_2D,q,0,Y):r.framebufferTexture2D(r.FRAMEBUFFER,r.DEPTH_ATTACHMENT,r.TEXTURE_2D,q,0);else if(b.depthTexture.format===$i)Ve(b)?a.framebufferTexture2DMultisampleEXT(r.FRAMEBUFFER,r.DEPTH_STENCIL_ATTACHMENT,r.TEXTURE_2D,q,0,Y):r.framebufferTexture2D(r.FRAMEBUFFER,r.DEPTH_STENCIL_ATTACHMENT,r.TEXTURE_2D,q,0);else throw new Error("Unknown depthTexture format")}function ne(A){let b=i.get(A),G=A.isWebGLCubeRenderTarget===!0;if(A.depthTexture&&!b.__autoAllocateDepthBuffer){if(G)throw new Error("target.depthTexture not supported in Cube render targets");he(b.__webglFramebuffer,A)}else if(G){b.__webglDepthbuffer=[];for(let q=0;q<6;q++)t.bindFramebuffer(r.FRAMEBUFFER,b.__webglFramebuffer[q]),b.__webglDepthbuffer[q]=r.createRenderbuffer(),ee(b.__webglDepthbuffer[q],A,!1)}else t.bindFramebuffer(r.FRAMEBUFFER,b.__webglFramebuffer),b.__webglDepthbuffer=r.createRenderbuffer(),ee(b.__webglDepthbuffer,A,!1);t.bindFramebuffer(r.FRAMEBUFFER,null)}function Be(A,b,G){let q=i.get(A);b!==void 0&&H(q.__webglFramebuffer,A,A.texture,r.COLOR_ATTACHMENT0,r.TEXTURE_2D,0),G!==void 0&&ne(A)}function ke(A){let b=A.texture,G=i.get(A),q=i.get(b);A.addEventListener("dispose",P);let Y=A.textures,K=A.isWebGLCubeRenderTarget===!0,xe=Y.length>1;if(xe||(q.__webglTexture===void 0&&(q.__webglTexture=r.createTexture()),q.__version=b.version,s.memory.textures++),K){G.__webglFramebuffer=[];for(let ce=0;ce<6;ce++)if(b.mipmaps&&b.mipmaps.length>0){G.__webglFramebuffer[ce]=[];for(let ae=0;ae<b.mipmaps.length;ae++)G.__webglFramebuffer[ce][ae]=r.createFramebuffer()}else G.__webglFramebuffer[ce]=r.createFramebuffer()}else{if(b.mipmaps&&b.mipmaps.length>0){G.__webglFramebuffer=[];for(let ce=0;ce<b.mipmaps.length;ce++)G.__webglFramebuffer[ce]=r.createFramebuffer()}else G.__webglFramebuffer=r.createFramebuffer();if(xe)for(let ce=0,ae=Y.length;ce<ae;ce++){let Ie=i.get(Y[ce]);Ie.__webglTexture===void 0&&(Ie.__webglTexture=r.createTexture(),s.memory.textures++)}if(A.samples>0&&Ve(A)===!1){G.__webglMultisampledFramebuffer=r.createFramebuffer(),G.__webglColorRenderbuffer=[],t.bindFramebuffer(r.FRAMEBUFFER,G.__webglMultisampledFramebuffer);for(let ce=0;ce<Y.length;ce++){let ae=Y[ce];G.__webglColorRenderbuffer[ce]=r.createRenderbuffer(),r.bindRenderbuffer(r.RENDERBUFFER,G.__webglColorRenderbuffer[ce]);let Ie=o.convert(ae.format,ae.colorSpace),re=o.convert(ae.type),ge=S(ae.internalFormat,Ie,re,ae.colorSpace,A.isXRRenderTarget===!0),He=Re(A);r.renderbufferStorageMultisample(r.RENDERBUFFER,He,ge,A.width,A.height),r.framebufferRenderbuffer(r.FRAMEBUFFER,r.COLOR_ATTACHMENT0+ce,r.RENDERBUFFER,G.__webglColorRenderbuffer[ce])}r.bindRenderbuffer(r.RENDERBUFFER,null),A.depthBuffer&&(G.__webglDepthRenderbuffer=r.createRenderbuffer(),ee(G.__webglDepthRenderbuffer,A,!0)),t.bindFramebuffer(r.FRAMEBUFFER,null)}}if(K){t.bindTexture(r.TEXTURE_CUBE_MAP,q.__webglTexture),Q(r.TEXTURE_CUBE_MAP,b);for(let ce=0;ce<6;ce++)if(b.mipmaps&&b.mipmaps.length>0)for(let ae=0;ae<b.mipmaps.length;ae++)H(G.__webglFramebuffer[ce][ae],A,b,r.COLOR_ATTACHMENT0,r.TEXTURE_CUBE_MAP_POSITIVE_X+ce,ae);else H(G.__webglFramebuffer[ce],A,b,r.COLOR_ATTACHMENT0,r.TEXTURE_CUBE_MAP_POSITIVE_X+ce,0);m(b)&&f(r.TEXTURE_CUBE_MAP),t.unbindTexture()}else if(xe){for(let ce=0,ae=Y.length;ce<ae;ce++){let Ie=Y[ce],re=i.get(Ie);t.bindTexture(r.TEXTURE_2D,re.__webglTexture),Q(r.TEXTURE_2D,Ie),H(G.__webglFramebuffer,A,Ie,r.COLOR_ATTACHMENT0+ce,r.TEXTURE_2D,0),m(Ie)&&f(r.TEXTURE_2D)}t.unbindTexture()}else{let ce=r.TEXTURE_2D;if((A.isWebGL3DRenderTarget||A.isWebGLArrayRenderTarget)&&(ce=A.isWebGL3DRenderTarget?r.TEXTURE_3D:r.TEXTURE_2D_ARRAY),t.bindTexture(ce,q.__webglTexture),Q(ce,b),b.mipmaps&&b.mipmaps.length>0)for(let ae=0;ae<b.mipmaps.length;ae++)H(G.__webglFramebuffer[ae],A,b,r.COLOR_ATTACHMENT0,ce,ae);else H(G.__webglFramebuffer,A,b,r.COLOR_ATTACHMENT0,ce,0);m(b)&&f(ce),t.unbindTexture()}A.depthBuffer&&ne(A)}function O(A){let b=A.textures;for(let G=0,q=b.length;G<q;G++){let Y=b[G];if(m(Y)){let K=A.isWebGLCubeRenderTarget?r.TEXTURE_CUBE_MAP:r.TEXTURE_2D,xe=i.get(Y).__webglTexture;t.bindTexture(K,xe),f(K),t.unbindTexture()}}}let ot=[],ve=[];function it(A){if(A.samples>0){if(Ve(A)===!1){let b=A.textures,G=A.width,q=A.height,Y=r.COLOR_BUFFER_BIT,K=A.stencilBuffer?r.DEPTH_STENCIL_ATTACHMENT:r.DEPTH_ATTACHMENT,xe=i.get(A),ce=b.length>1;if(ce)for(let ae=0;ae<b.length;ae++)t.bindFramebuffer(r.FRAMEBUFFER,xe.__webglMultisampledFramebuffer),r.framebufferRenderbuffer(r.FRAMEBUFFER,r.COLOR_ATTACHMENT0+ae,r.RENDERBUFFER,null),t.bindFramebuffer(r.FRAMEBUFFER,xe.__webglFramebuffer),r.framebufferTexture2D(r.DRAW_FRAMEBUFFER,r.COLOR_ATTACHMENT0+ae,r.TEXTURE_2D,null,0);t.bindFramebuffer(r.READ_FRAMEBUFFER,xe.__webglMultisampledFramebuffer),t.bindFramebuffer(r.DRAW_FRAMEBUFFER,xe.__webglFramebuffer);for(let ae=0;ae<b.length;ae++){if(A.resolveDepthBuffer&&(A.depthBuffer&&(Y|=r.DEPTH_BUFFER_BIT),A.stencilBuffer&&A.resolveStencilBuffer&&(Y|=r.STENCIL_BUFFER_BIT)),ce){r.framebufferRenderbuffer(r.READ_FRAMEBUFFER,r.COLOR_ATTACHMENT0,r.RENDERBUFFER,xe.__webglColorRenderbuffer[ae]);let Ie=i.get(b[ae]).__webglTexture;r.framebufferTexture2D(r.DRAW_FRAMEBUFFER,r.COLOR_ATTACHMENT0,r.TEXTURE_2D,Ie,0)}r.blitFramebuffer(0,0,G,q,0,0,G,q,Y,r.NEAREST),c===!0&&(ot.length=0,ve.length=0,ot.push(r.COLOR_ATTACHMENT0+ae),A.depthBuffer&&A.resolveDepthBuffer===!1&&(ot.push(K),ve.push(K),r.invalidateFramebuffer(r.DRAW_FRAMEBUFFER,ve)),r.invalidateFramebuffer(r.READ_FRAMEBUFFER,ot))}if(t.bindFramebuffer(r.READ_FRAMEBUFFER,null),t.bindFramebuffer(r.DRAW_FRAMEBUFFER,null),ce)for(let ae=0;ae<b.length;ae++){t.bindFramebuffer(r.FRAMEBUFFER,xe.__webglMultisampledFramebuffer),r.framebufferRenderbuffer(r.FRAMEBUFFER,r.COLOR_ATTACHMENT0+ae,r.RENDERBUFFER,xe.__webglColorRenderbuffer[ae]);let Ie=i.get(b[ae]).__webglTexture;t.bindFramebuffer(r.FRAMEBUFFER,xe.__webglFramebuffer),r.framebufferTexture2D(r.DRAW_FRAMEBUFFER,r.COLOR_ATTACHMENT0+ae,r.TEXTURE_2D,Ie,0)}t.bindFramebuffer(r.DRAW_FRAMEBUFFER,xe.__webglMultisampledFramebuffer)}else if(A.depthBuffer&&A.resolveDepthBuffer===!1&&c){let b=A.stencilBuffer?r.DEPTH_STENCIL_ATTACHMENT:r.DEPTH_ATTACHMENT;r.invalidateFramebuffer(r.DRAW_FRAMEBUFFER,[b])}}}function Re(A){return Math.min(n.maxSamples,A.samples)}function Ve(A){let b=i.get(A);return A.samples>0&&e.has("WEBGL_multisampled_render_to_texture")===!0&&b.__useRenderToTexture!==!1}function Ue(A){let b=s.render.frame;d.get(A)!==b&&(d.set(A,b),A.update())}function Ge(A,b){let G=A.colorSpace,q=A.format,Y=A.type;return A.isCompressedTexture===!0||A.isVideoTexture===!0||G!==ei&&G!==hi&&(Oe.getTransfer(G)===$e?(q!==Mt||Y!==Ut)&&console.warn("THREE.WebGLTextures: sRGB encoded textures have to use RGBAFormat and UnsignedByteType."):console.error("THREE.WebGLTextures: Unsupported texture color space:",G)),b}function pt(A){return typeof VideoFrame<"u"&&A instanceof VideoFrame?(l.width=A.displayWidth,l.height=A.displayHeight):(l.width=A.width,l.height=A.height),l}this.allocateTextureUnit=I,this.resetTextureUnits=v,this.setTexture2D=E,this.setTexture2DArray=N,this.setTexture3D=W,this.setTextureCube=X,this.rebindTextures=Be,this.setupRenderTarget=ke,this.updateRenderTargetMipmap=O,this.updateMultisampleRenderTarget=it,this.setupDepthRenderbuffer=ne,this.setupFrameBufferTexture=H,this.useMultisampledRTT=Ve}function Hp(r,e){function t(i,n=hi){let o,s=Oe.getTransfer(n);if(i===Ut)return r.UNSIGNED_BYTE;if(i===Ko)return r.UNSIGNED_SHORT_4_4_4_4;if(i===Zo)return r.UNSIGNED_SHORT_5_5_5_1;if(i===bd)return r.UNSIGNED_INT_5_9_9_9_REV;if(i===yd)return r.BYTE;if(i===Sd)return r.SHORT;if(i===$o)return r.UNSIGNED_SHORT;if(i===Yo)return r.INT;if(i===yi)return r.UNSIGNED_INT;if(i===oi)return r.FLOAT;if(i===fr)return r.HALF_FLOAT;if(i===Rd)return r.ALPHA;if(i===Md)return r.RGB;if(i===Mt)return r.RGBA;if(i===wd)return r.LUMINANCE;if(i===Td)return r.LUMINANCE_ALPHA;if(i===Oi)return r.DEPTH_COMPONENT;if(i===$i)return r.DEPTH_STENCIL;if(i===Ed)return r.RED;if(i===Jo)return r.RED_INTEGER;if(i===Ad)return r.RG;if(i===Qo)return r.RG_INTEGER;if(i===es)return r.RGBA_INTEGER;if(i===ts||i===is||i===rs||i===ns)if(s===$e)if(o=e.get("WEBGL_compressed_texture_s3tc_srgb"),o!==null){if(i===ts)return o.COMPRESSED_SRGB_S3TC_DXT1_EXT;if(i===is)return o.COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT;if(i===rs)return o.COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT;if(i===ns)return o.COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT}else return null;else if(o=e.get("WEBGL_compressed_texture_s3tc"),o!==null){if(i===ts)return o.COMPRESSED_RGB_S3TC_DXT1_EXT;if(i===is)return o.COMPRESSED_RGBA_S3TC_DXT1_EXT;if(i===rs)return o.COMPRESSED_RGBA_S3TC_DXT3_EXT;if(i===ns)return o.COMPRESSED_RGBA_S3TC_DXT5_EXT}else return null;if(i===Ma||i===wa||i===Ta||i===Ea)if(o=e.get("WEBGL_compressed_texture_pvrtc"),o!==null){if(i===Ma)return o.COMPRESSED_RGB_PVRTC_4BPPV1_IMG;if(i===wa)return o.COMPRESSED_RGB_PVRTC_2BPPV1_IMG;if(i===Ta)return o.COMPRESSED_RGBA_PVRTC_4BPPV1_IMG;if(i===Ea)return o.COMPRESSED_RGBA_PVRTC_2BPPV1_IMG}else return null;if(i===Aa||i===Ca||i===Pa)if(o=e.get("WEBGL_compressed_texture_etc"),o!==null){if(i===Aa||i===Ca)return s===$e?o.COMPRESSED_SRGB8_ETC2:o.COMPRESSED_RGB8_ETC2;if(i===Pa)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ETC2_EAC:o.COMPRESSED_RGBA8_ETC2_EAC}else return null;if(i===Ia||i===Da||i===La||i===Ua||i===Fa||i===Oa||i===Na||i===Ba||i===ka||i===za||i===Va||i===Ga||i===Wa||i===Ha)if(o=e.get("WEBGL_compressed_texture_astc"),o!==null){if(i===Ia)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_4x4_KHR:o.COMPRESSED_RGBA_ASTC_4x4_KHR;if(i===Da)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_5x4_KHR:o.COMPRESSED_RGBA_ASTC_5x4_KHR;if(i===La)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_5x5_KHR:o.COMPRESSED_RGBA_ASTC_5x5_KHR;if(i===Ua)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_6x5_KHR:o.COMPRESSED_RGBA_ASTC_6x5_KHR;if(i===Fa)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_6x6_KHR:o.COMPRESSED_RGBA_ASTC_6x6_KHR;if(i===Oa)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_8x5_KHR:o.COMPRESSED_RGBA_ASTC_8x5_KHR;if(i===Na)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_8x6_KHR:o.COMPRESSED_RGBA_ASTC_8x6_KHR;if(i===Ba)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_8x8_KHR:o.COMPRESSED_RGBA_ASTC_8x8_KHR;if(i===ka)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_10x5_KHR:o.COMPRESSED_RGBA_ASTC_10x5_KHR;if(i===za)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_10x6_KHR:o.COMPRESSED_RGBA_ASTC_10x6_KHR;if(i===Va)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_10x8_KHR:o.COMPRESSED_RGBA_ASTC_10x8_KHR;if(i===Ga)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_10x10_KHR:o.COMPRESSED_RGBA_ASTC_10x10_KHR;if(i===Wa)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_12x10_KHR:o.COMPRESSED_RGBA_ASTC_12x10_KHR;if(i===Ha)return s===$e?o.COMPRESSED_SRGB8_ALPHA8_ASTC_12x12_KHR:o.COMPRESSED_RGBA_ASTC_12x12_KHR}else return null;if(i===os||i===ja||i===Xa)if(o=e.get("EXT_texture_compression_bptc"),o!==null){if(i===os)return s===$e?o.COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT:o.COMPRESSED_RGBA_BPTC_UNORM_EXT;if(i===ja)return o.COMPRESSED_RGB_BPTC_SIGNED_FLOAT_EXT;if(i===Xa)return o.COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT_EXT}else return null;if(i===Cd||i===qa||i===$a||i===Ya)if(o=e.get("EXT_texture_compression_rgtc"),o!==null){if(i===os)return o.COMPRESSED_RED_RGTC1_EXT;if(i===qa)return o.COMPRESSED_SIGNED_RED_RGTC1_EXT;if(i===$a)return o.COMPRESSED_RED_GREEN_RGTC2_EXT;if(i===Ya)return o.COMPRESSED_SIGNED_RED_GREEN_RGTC2_EXT}else return null;return i===Fi?r.UNSIGNED_INT_24_8:r[i]!==void 0?r[i]:null}return{convert:t}}var Gs=class extends Bt{constructor(e=[]){super(),this.isArrayCamera=!0,this.cameras=e}};var Vr=class extends vt{constructor(){super(),this.isGroup=!0,this.type="Group"}};var fv={type:"move"},En=class{constructor(){this._targetRay=null,this._grip=null,this._hand=null}getHandSpace(){return this._hand===null&&(this._hand=new Vr,this._hand.matrixAutoUpdate=!1,this._hand.visible=!1,this._hand.joints={},this._hand.inputState={pinching:!1}),this._hand}getTargetRaySpace(){return this._targetRay===null&&(this._targetRay=new Vr,this._targetRay.matrixAutoUpdate=!1,this._targetRay.visible=!1,this._targetRay.hasLinearVelocity=!1,this._targetRay.linearVelocity=new D,this._targetRay.hasAngularVelocity=!1,this._targetRay.angularVelocity=new D),this._targetRay}getGripSpace(){return this._grip===null&&(this._grip=new Vr,this._grip.matrixAutoUpdate=!1,this._grip.visible=!1,this._grip.hasLinearVelocity=!1,this._grip.linearVelocity=new D,this._grip.hasAngularVelocity=!1,this._grip.angularVelocity=new D),this._grip}dispatchEvent(e){return this._targetRay!==null&&this._targetRay.dispatchEvent(e),this._grip!==null&&this._grip.dispatchEvent(e),this._hand!==null&&this._hand.dispatchEvent(e),this}connect(e){if(e&&e.hand){let t=this._hand;if(t)for(let i of e.hand.values())this._getHandJoint(t,i)}return this.dispatchEvent({type:"connected",data:e}),this}disconnect(e){return this.dispatchEvent({type:"disconnected",data:e}),this._targetRay!==null&&(this._targetRay.visible=!1),this._grip!==null&&(this._grip.visible=!1),this._hand!==null&&(this._hand.visible=!1),this}update(e,t,i){let n=null,o=null,s=null,a=this._targetRay,c=this._grip,l=this._hand;if(e&&t.session.visibilityState!=="visible-blurred"){if(l&&e.hand){s=!0;for(let g of e.hand.values()){let m=t.getJointPose(g,i),f=this._getHandJoint(l,g);m!==null&&(f.matrix.fromArray(m.transform.matrix),f.matrix.decompose(f.position,f.rotation,f.scale),f.matrixWorldNeedsUpdate=!0,f.jointRadius=m.radius),f.visible=m!==null}let d=l.joints["index-finger-tip"],u=l.joints["thumb-tip"],h=d.position.distanceTo(u.position),p=.02,x=.005;l.inputState.pinching&&h>p+x?(l.inputState.pinching=!1,this.dispatchEvent({type:"pinchend",handedness:e.handedness,target:this})):!l.inputState.pinching&&h<=p-x&&(l.inputState.pinching=!0,this.dispatchEvent({type:"pinchstart",handedness:e.handedness,target:this}))}else c!==null&&e.gripSpace&&(o=t.getPose(e.gripSpace,i),o!==null&&(c.matrix.fromArray(o.transform.matrix),c.matrix.decompose(c.position,c.rotation,c.scale),c.matrixWorldNeedsUpdate=!0,o.linearVelocity?(c.hasLinearVelocity=!0,c.linearVelocity.copy(o.linearVelocity)):c.hasLinearVelocity=!1,o.angularVelocity?(c.hasAngularVelocity=!0,c.angularVelocity.copy(o.angularVelocity)):c.hasAngularVelocity=!1));a!==null&&(n=t.getPose(e.targetRaySpace,i),n===null&&o!==null&&(n=o),n!==null&&(a.matrix.fromArray(n.transform.matrix),a.matrix.decompose(a.position,a.rotation,a.scale),a.matrixWorldNeedsUpdate=!0,n.linearVelocity?(a.hasLinearVelocity=!0,a.linearVelocity.copy(n.linearVelocity)):a.hasLinearVelocity=!1,n.angularVelocity?(a.hasAngularVelocity=!0,a.angularVelocity.copy(n.angularVelocity)):a.hasAngularVelocity=!1,this.dispatchEvent(fv)))}return a!==null&&(a.visible=n!==null),c!==null&&(c.visible=o!==null),l!==null&&(l.visible=s!==null),this}_getHandJoint(e,t){if(e.joints[t.jointName]===void 0){let i=new Vr;i.matrixAutoUpdate=!1,i.visible=!1,e.joints[t.jointName]=i,e.add(i)}return e.joints[t.jointName]}};var pv=`
void main() {

	gl_Position = vec4( position, 1.0 );

}`,mv=`
uniform sampler2DArray depthColor;
uniform float depthWidth;
uniform float depthHeight;

void main() {

	vec2 coord = vec2( gl_FragCoord.x / depthWidth, gl_FragCoord.y / depthHeight );

	if ( coord.x >= 1.0 ) {

		gl_FragDepth = texture( depthColor, vec3( coord.x - 1.0, coord.y, 1 ) ).r;

	} else {

		gl_FragDepth = texture( depthColor, vec3( coord.x, coord.y, 0 ) ).r;

	}

}`,Ws=class{constructor(){this.texture=null,this.mesh=null,this.depthNear=0,this.depthFar=0}init(e,t,i){if(this.texture===null){let n=new St,o=e.properties.get(n);o.__webglTexture=t.texture,(t.depthNear!=i.depthNear||t.depthFar!=i.depthFar)&&(this.depthNear=t.depthNear,this.depthFar=t.depthFar),this.texture=n}}render(e,t){if(this.texture!==null){if(this.mesh===null){let i=t.cameras[0].viewport,n=new ft({vertexShader:pv,fragmentShader:mv,uniforms:{depthColor:{value:this.texture},depthWidth:{value:i.z},depthHeight:{value:i.w}}});this.mesh=new Xe(new ci(20,20),n)}e.render(this.mesh,t)}}reset(){this.texture=null,this.mesh=null}};var Hs=class extends Qt{constructor(e,t){super();let i=this,n=null,o=1,s=null,a="local-floor",c=1,l=null,d=null,u=null,h=null,p=null,x=null,g=new Ws,m=t.getContextAttributes(),f=null,S=null,_=[],R=[],P=new be,w=null,T=new Bt;T.layers.enable(1),T.viewport=new ct;let C=new Bt;C.layers.enable(2),C.viewport=new ct;let y=[T,C],v=new Gs;v.layers.enable(1),v.layers.enable(2);let I=null,F=null;this.cameraAutoUpdate=!0,this.enabled=!1,this.isPresenting=!1,this.getController=function(H){let ee=_[H];return ee===void 0&&(ee=new En,_[H]=ee),ee.getTargetRaySpace()},this.getControllerGrip=function(H){let ee=_[H];return ee===void 0&&(ee=new En,_[H]=ee),ee.getGripSpace()},this.getHand=function(H){let ee=_[H];return ee===void 0&&(ee=new En,_[H]=ee),ee.getHandSpace()};function E(H){let ee=R.indexOf(H.inputSource);if(ee===-1)return;let he=_[ee];he!==void 0&&(he.update(H.inputSource,H.frame,l||s),he.dispatchEvent({type:H.type,data:H.inputSource}))}function N(){n.removeEventListener("select",E),n.removeEventListener("selectstart",E),n.removeEventListener("selectend",E),n.removeEventListener("squeeze",E),n.removeEventListener("squeezestart",E),n.removeEventListener("squeezeend",E),n.removeEventListener("end",N),n.removeEventListener("inputsourceschange",W);for(let H=0;H<_.length;H++){let ee=R[H];ee!==null&&(R[H]=null,_[H].disconnect(ee))}I=null,F=null,g.reset(),e.setRenderTarget(f),p=null,h=null,u=null,n=null,S=null,tt.stop(),i.isPresenting=!1,e.setPixelRatio(w),e.setSize(P.width,P.height,!1),i.dispatchEvent({type:"sessionend"})}this.setFramebufferScaleFactor=function(H){o=H,i.isPresenting===!0&&console.warn("THREE.WebXRManager: Cannot change framebuffer scale while presenting.")},this.setReferenceSpaceType=function(H){a=H,i.isPresenting===!0&&console.warn("THREE.WebXRManager: Cannot change reference space type while presenting.")},this.getReferenceSpace=function(){return l||s},this.setReferenceSpace=function(H){l=H},this.getBaseLayer=function(){return h!==null?h:p},this.getBinding=function(){return u},this.getFrame=function(){return x},this.getSession=function(){return n},this.setSession=async function(H){if(n=H,n!==null){if(f=e.getRenderTarget(),n.addEventListener("select",E),n.addEventListener("selectstart",E),n.addEventListener("selectend",E),n.addEventListener("squeeze",E),n.addEventListener("squeezestart",E),n.addEventListener("squeezeend",E),n.addEventListener("end",N),n.addEventListener("inputsourceschange",W),m.xrCompatible!==!0&&await t.makeXRCompatible(),w=e.getPixelRatio(),e.getSize(P),n.renderState.layers===void 0){let ee={antialias:m.antialias,alpha:!0,depth:m.depth,stencil:m.stencil,framebufferScaleFactor:o};p=new XRWebGLLayer(n,t,ee),n.updateRenderState({baseLayer:p}),e.setPixelRatio(1),e.setSize(p.framebufferWidth,p.framebufferHeight,!1),S=new Ot(p.framebufferWidth,p.framebufferHeight,{format:Mt,type:Ut,colorSpace:e.outputColorSpace,stencilBuffer:m.stencil})}else{let ee=null,he=null,ne=null;m.depth&&(ne=m.stencil?t.DEPTH24_STENCIL8:t.DEPTH_COMPONENT24,ee=m.stencil?$i:Oi,he=m.stencil?Fi:yi);let Be={colorFormat:t.RGBA8,depthFormat:ne,scaleFactor:o};u=new XRWebGLBinding(n,t),h=u.createProjectionLayer(Be),n.updateRenderState({layers:[h]}),e.setPixelRatio(1),e.setSize(h.textureWidth,h.textureHeight,!1),S=new Ot(h.textureWidth,h.textureHeight,{format:Mt,type:Ut,depthTexture:new wn(h.textureWidth,h.textureHeight,he,void 0,void 0,void 0,void 0,void 0,void 0,ee),stencilBuffer:m.stencil,colorSpace:e.outputColorSpace,samples:m.antialias?4:0,resolveDepthBuffer:h.ignoreDepthValues===!1})}S.isXRRenderTarget=!0,this.setFoveation(c),l=null,s=await n.requestReferenceSpace(a),tt.setContext(n),tt.start(),i.isPresenting=!0,i.dispatchEvent({type:"sessionstart"})}},this.getEnvironmentBlendMode=function(){if(n!==null)return n.environmentBlendMode};function W(H){for(let ee=0;ee<H.removed.length;ee++){let he=H.removed[ee],ne=R.indexOf(he);ne>=0&&(R[ne]=null,_[ne].disconnect(he))}for(let ee=0;ee<H.added.length;ee++){let he=H.added[ee],ne=R.indexOf(he);if(ne===-1){for(let ke=0;ke<_.length;ke++)if(ke>=R.length){R.push(he),ne=ke;break}else if(R[ke]===null){R[ke]=he,ne=ke;break}if(ne===-1)break}let Be=_[ne];Be&&Be.connect(he)}}let X=new D,te=new D;function V(H,ee,he){X.setFromMatrixPosition(ee.matrixWorld),te.setFromMatrixPosition(he.matrixWorld);let ne=X.distanceTo(te),Be=ee.projectionMatrix.elements,ke=he.projectionMatrix.elements,O=Be[14]/(Be[10]-1),ot=Be[14]/(Be[10]+1),ve=(Be[9]+1)/Be[5],it=(Be[9]-1)/Be[5],Re=(Be[8]-1)/Be[0],Ve=(ke[8]+1)/ke[0],Ue=O*Re,Ge=O*Ve,pt=ne/(-Re+Ve),A=pt*-Re;ee.matrixWorld.decompose(H.position,H.quaternion,H.scale),H.translateX(A),H.translateZ(pt),H.matrixWorld.compose(H.position,H.quaternion,H.scale),H.matrixWorldInverse.copy(H.matrixWorld).invert();let b=O+pt,G=ot+pt,q=Ue-A,Y=Ge+(ne-A),K=ve*ot/G*b,xe=it*ot/G*b;H.projectionMatrix.makePerspective(q,Y,K,xe,b,G),H.projectionMatrixInverse.copy(H.projectionMatrix).invert()}function J(H,ee){ee===null?H.matrixWorld.copy(H.matrix):H.matrixWorld.multiplyMatrices(ee.matrixWorld,H.matrix),H.matrixWorldInverse.copy(H.matrixWorld).invert()}this.updateCamera=function(H){if(n===null)return;g.texture!==null&&(H.near=g.depthNear,H.far=g.depthFar),v.near=C.near=T.near=H.near,v.far=C.far=T.far=H.far,(I!==v.near||F!==v.far)&&(n.updateRenderState({depthNear:v.near,depthFar:v.far}),I=v.near,F=v.far,T.near=I,T.far=F,C.near=I,C.far=F,T.updateProjectionMatrix(),C.updateProjectionMatrix(),H.updateProjectionMatrix());let ee=H.parent,he=v.cameras;J(v,ee);for(let ne=0;ne<he.length;ne++)J(he[ne],ee);he.length===2?V(v,T,C):v.projectionMatrix.copy(T.projectionMatrix),Q(H,v,ee)};function Q(H,ee,he){he===null?H.matrix.copy(ee.matrixWorld):(H.matrix.copy(he.matrixWorld),H.matrix.invert(),H.matrix.multiply(ee.matrixWorld)),H.matrix.decompose(H.position,H.quaternion,H.scale),H.updateMatrixWorld(!0),H.projectionMatrix.copy(ee.projectionMatrix),H.projectionMatrixInverse.copy(ee.projectionMatrixInverse),H.isPerspectiveCamera&&(H.fov=Kn*2*Math.atan(1/H.projectionMatrix.elements[5]),H.zoom=1)}this.getCamera=function(){return v},this.getFoveation=function(){if(!(h===null&&p===null))return c},this.setFoveation=function(H){c=H,h!==null&&(h.fixedFoveation=H),p!==null&&p.fixedFoveation!==void 0&&(p.fixedFoveation=H)},this.hasDepthSensing=function(){return g.texture!==null};let me=null;function Ne(H,ee){if(d=ee.getViewerPose(l||s),x=ee,d!==null){let he=d.views;p!==null&&(e.setRenderTargetFramebuffer(S,p.framebuffer),e.setRenderTarget(S));let ne=!1;he.length!==v.cameras.length&&(v.cameras.length=0,ne=!0);for(let ke=0;ke<he.length;ke++){let O=he[ke],ot=null;if(p!==null)ot=p.getViewport(O);else{let it=u.getViewSubImage(h,O);ot=it.viewport,ke===0&&(e.setRenderTargetTextures(S,it.colorTexture,h.ignoreDepthValues?void 0:it.depthStencilTexture),e.setRenderTarget(S))}let ve=y[ke];ve===void 0&&(ve=new Bt,ve.layers.enable(ke),ve.viewport=new ct,y[ke]=ve),ve.matrix.fromArray(O.transform.matrix),ve.matrix.decompose(ve.position,ve.quaternion,ve.scale),ve.projectionMatrix.fromArray(O.projectionMatrix),ve.projectionMatrixInverse.copy(ve.projectionMatrix).invert(),ve.viewport.set(ot.x,ot.y,ot.width,ot.height),ke===0&&(v.matrix.copy(ve.matrix),v.matrix.decompose(v.position,v.quaternion,v.scale)),ne===!0&&v.cameras.push(ve)}let Be=n.enabledFeatures;if(Be&&Be.includes("depth-sensing")){let ke=u.getDepthInformation(he[0]);ke&&ke.isValid&&ke.texture&&g.init(e,ke,n.renderState)}}for(let he=0;he<_.length;he++){let ne=R[he],Be=_[he];ne!==null&&Be!==void 0&&Be.update(ne,ee,l||s)}g.render(e,v),me&&me(H,ee),ee.detectedPlanes&&i.dispatchEvent({type:"planesdetected",data:ee}),x=null}let tt=new Ds;tt.setAnimationLoop(Ne),this.setAnimationLoop=function(H){me=H},this.dispose=function(){}}};var Gr=new ii,gv=new Ee;function jp(r,e){function t(m,f){m.matrixAutoUpdate===!0&&m.updateMatrix(),f.value.copy(m.matrix)}function i(m,f){f.color.getRGB(m.fogColor.value,ms(r)),f.isFog?(m.fogNear.value=f.near,m.fogFar.value=f.far):f.isFogExp2&&(m.fogDensity.value=f.density)}function n(m,f,S,_,R){f.isMeshBasicMaterial||f.isMeshLambertMaterial?o(m,f):f.isMeshToonMaterial?(o(m,f),u(m,f)):f.isMeshPhongMaterial?(o(m,f),d(m,f)):f.isMeshStandardMaterial?(o(m,f),h(m,f),f.isMeshPhysicalMaterial&&p(m,f,R)):f.isMeshMatcapMaterial?(o(m,f),x(m,f)):f.isMeshDepthMaterial?o(m,f):f.isMeshDistanceMaterial?(o(m,f),g(m,f)):f.isMeshNormalMaterial?o(m,f):f.isLineBasicMaterial?(s(m,f),f.isLineDashedMaterial&&a(m,f)):f.isPointsMaterial?c(m,f,S,_):f.isSpriteMaterial?l(m,f):f.isShadowMaterial?(m.color.value.copy(f.color),m.opacity.value=f.opacity):f.isShaderMaterial&&(f.uniformsNeedUpdate=!1)}function o(m,f){m.opacity.value=f.opacity,f.color&&m.diffuse.value.copy(f.color),f.emissive&&m.emissive.value.copy(f.emissive).multiplyScalar(f.emissiveIntensity),f.map&&(m.map.value=f.map,t(f.map,m.mapTransform)),f.alphaMap&&(m.alphaMap.value=f.alphaMap,t(f.alphaMap,m.alphaMapTransform)),f.bumpMap&&(m.bumpMap.value=f.bumpMap,t(f.bumpMap,m.bumpMapTransform),m.bumpScale.value=f.bumpScale,f.side===ut&&(m.bumpScale.value*=-1)),f.normalMap&&(m.normalMap.value=f.normalMap,t(f.normalMap,m.normalMapTransform),m.normalScale.value.copy(f.normalScale),f.side===ut&&m.normalScale.value.negate()),f.displacementMap&&(m.displacementMap.value=f.displacementMap,t(f.displacementMap,m.displacementMapTransform),m.displacementScale.value=f.displacementScale,m.displacementBias.value=f.displacementBias),f.emissiveMap&&(m.emissiveMap.value=f.emissiveMap,t(f.emissiveMap,m.emissiveMapTransform)),f.specularMap&&(m.specularMap.value=f.specularMap,t(f.specularMap,m.specularMapTransform)),f.alphaTest>0&&(m.alphaTest.value=f.alphaTest);let S=e.get(f),_=S.envMap,R=S.envMapRotation;if(_&&(m.envMap.value=_,Gr.copy(R),Gr.x*=-1,Gr.y*=-1,Gr.z*=-1,_.isCubeTexture&&_.isRenderTargetTexture===!1&&(Gr.y*=-1,Gr.z*=-1),m.envMapRotation.value.setFromMatrix4(gv.makeRotationFromEuler(Gr)),m.flipEnvMap.value=_.isCubeTexture&&_.isRenderTargetTexture===!1?-1:1,m.reflectivity.value=f.reflectivity,m.ior.value=f.ior,m.refractionRatio.value=f.refractionRatio),f.lightMap){m.lightMap.value=f.lightMap;let P=r._useLegacyLights===!0?Math.PI:1;m.lightMapIntensity.value=f.lightMapIntensity*P,t(f.lightMap,m.lightMapTransform)}f.aoMap&&(m.aoMap.value=f.aoMap,m.aoMapIntensity.value=f.aoMapIntensity,t(f.aoMap,m.aoMapTransform))}function s(m,f){m.diffuse.value.copy(f.color),m.opacity.value=f.opacity,f.map&&(m.map.value=f.map,t(f.map,m.mapTransform))}function a(m,f){m.dashSize.value=f.dashSize,m.totalSize.value=f.dashSize+f.gapSize,m.scale.value=f.scale}function c(m,f,S,_){m.diffuse.value.copy(f.color),m.opacity.value=f.opacity,m.size.value=f.size*S,m.scale.value=_*.5,f.map&&(m.map.value=f.map,t(f.map,m.uvTransform)),f.alphaMap&&(m.alphaMap.value=f.alphaMap,t(f.alphaMap,m.alphaMapTransform)),f.alphaTest>0&&(m.alphaTest.value=f.alphaTest)}function l(m,f){m.diffuse.value.copy(f.color),m.opacity.value=f.opacity,m.rotation.value=f.rotation,f.map&&(m.map.value=f.map,t(f.map,m.mapTransform)),f.alphaMap&&(m.alphaMap.value=f.alphaMap,t(f.alphaMap,m.alphaMapTransform)),f.alphaTest>0&&(m.alphaTest.value=f.alphaTest)}function d(m,f){m.specular.value.copy(f.specular),m.shininess.value=Math.max(f.shininess,1e-4)}function u(m,f){f.gradientMap&&(m.gradientMap.value=f.gradientMap)}function h(m,f){m.metalness.value=f.metalness,f.metalnessMap&&(m.metalnessMap.value=f.metalnessMap,t(f.metalnessMap,m.metalnessMapTransform)),m.roughness.value=f.roughness,f.roughnessMap&&(m.roughnessMap.value=f.roughnessMap,t(f.roughnessMap,m.roughnessMapTransform)),f.envMap&&(m.envMapIntensity.value=f.envMapIntensity)}function p(m,f,S){m.ior.value=f.ior,f.sheen>0&&(m.sheenColor.value.copy(f.sheenColor).multiplyScalar(f.sheen),m.sheenRoughness.value=f.sheenRoughness,f.sheenColorMap&&(m.sheenColorMap.value=f.sheenColorMap,t(f.sheenColorMap,m.sheenColorMapTransform)),f.sheenRoughnessMap&&(m.sheenRoughnessMap.value=f.sheenRoughnessMap,t(f.sheenRoughnessMap,m.sheenRoughnessMapTransform))),f.clearcoat>0&&(m.clearcoat.value=f.clearcoat,m.clearcoatRoughness.value=f.clearcoatRoughness,f.clearcoatMap&&(m.clearcoatMap.value=f.clearcoatMap,t(f.clearcoatMap,m.clearcoatMapTransform)),f.clearcoatRoughnessMap&&(m.clearcoatRoughnessMap.value=f.clearcoatRoughnessMap,t(f.clearcoatRoughnessMap,m.clearcoatRoughnessMapTransform)),f.clearcoatNormalMap&&(m.clearcoatNormalMap.value=f.clearcoatNormalMap,t(f.clearcoatNormalMap,m.clearcoatNormalMapTransform),m.clearcoatNormalScale.value.copy(f.clearcoatNormalScale),f.side===ut&&m.clearcoatNormalScale.value.negate())),f.dispersion>0&&(m.dispersion.value=f.dispersion),f.iridescence>0&&(m.iridescence.value=f.iridescence,m.iridescenceIOR.value=f.iridescenceIOR,m.iridescenceThicknessMinimum.value=f.iridescenceThicknessRange[0],m.iridescenceThicknessMaximum.value=f.iridescenceThicknessRange[1],f.iridescenceMap&&(m.iridescenceMap.value=f.iridescenceMap,t(f.iridescenceMap,m.iridescenceMapTransform)),f.iridescenceThicknessMap&&(m.iridescenceThicknessMap.value=f.iridescenceThicknessMap,t(f.iridescenceThicknessMap,m.iridescenceThicknessMapTransform))),f.transmission>0&&(m.transmission.value=f.transmission,m.transmissionSamplerMap.value=S.texture,m.transmissionSamplerSize.value.set(S.width,S.height),f.transmissionMap&&(m.transmissionMap.value=f.transmissionMap,t(f.transmissionMap,m.transmissionMapTransform)),m.thickness.value=f.thickness,f.thicknessMap&&(m.thicknessMap.value=f.thicknessMap,t(f.thicknessMap,m.thicknessMapTransform)),m.attenuationDistance.value=f.attenuationDistance,m.attenuationColor.value.copy(f.attenuationColor)),f.anisotropy>0&&(m.anisotropyVector.value.set(f.anisotropy*Math.cos(f.anisotropyRotation),f.anisotropy*Math.sin(f.anisotropyRotation)),f.anisotropyMap&&(m.anisotropyMap.value=f.anisotropyMap,t(f.anisotropyMap,m.anisotropyMapTransform))),m.specularIntensity.value=f.specularIntensity,m.specularColor.value.copy(f.specularColor),f.specularColorMap&&(m.specularColorMap.value=f.specularColorMap,t(f.specularColorMap,m.specularColorMapTransform)),f.specularIntensityMap&&(m.specularIntensityMap.value=f.specularIntensityMap,t(f.specularIntensityMap,m.specularIntensityMapTransform))}function x(m,f){f.matcap&&(m.matcap.value=f.matcap)}function g(m,f){let S=e.get(f).light;m.referencePosition.value.setFromMatrixPosition(S.matrixWorld),m.nearDistance.value=S.shadow.camera.near,m.farDistance.value=S.shadow.camera.far}return{refreshFogUniforms:i,refreshMaterialUniforms:n}}function Xp(r,e,t,i){let n={},o={},s=[],a=r.getParameter(r.MAX_UNIFORM_BUFFER_BINDINGS);function c(S,_){let R=_.program;i.uniformBlockBinding(S,R)}function l(S,_){let R=n[S.id];R===void 0&&(x(S),R=d(S),n[S.id]=R,S.addEventListener("dispose",m));let P=_.program;i.updateUBOMapping(S,P);let w=e.render.frame;o[S.id]!==w&&(h(S),o[S.id]=w)}function d(S){let _=u();S.__bindingPointIndex=_;let R=r.createBuffer(),P=S.__size,w=S.usage;return r.bindBuffer(r.UNIFORM_BUFFER,R),r.bufferData(r.UNIFORM_BUFFER,P,w),r.bindBuffer(r.UNIFORM_BUFFER,null),r.bindBufferBase(r.UNIFORM_BUFFER,_,R),R}function u(){for(let S=0;S<a;S++)if(s.indexOf(S)===-1)return s.push(S),S;return console.error("THREE.WebGLRenderer: Maximum number of simultaneously usable uniforms groups reached."),0}function h(S){let _=n[S.id],R=S.uniforms,P=S.__cache;r.bindBuffer(r.UNIFORM_BUFFER,_);for(let w=0,T=R.length;w<T;w++){let C=Array.isArray(R[w])?R[w]:[R[w]];for(let y=0,v=C.length;y<v;y++){let I=C[y];if(p(I,w,y,P)===!0){let F=I.__offset,E=Array.isArray(I.value)?I.value:[I.value],N=0;for(let W=0;W<E.length;W++){let X=E[W],te=g(X);typeof X=="number"||typeof X=="boolean"?(I.__data[0]=X,r.bufferSubData(r.UNIFORM_BUFFER,F+N,I.__data)):X.isMatrix3?(I.__data[0]=X.elements[0],I.__data[1]=X.elements[1],I.__data[2]=X.elements[2],I.__data[3]=0,I.__data[4]=X.elements[3],I.__data[5]=X.elements[4],I.__data[6]=X.elements[5],I.__data[7]=0,I.__data[8]=X.elements[6],I.__data[9]=X.elements[7],I.__data[10]=X.elements[8],I.__data[11]=0):(X.toArray(I.__data,N),N+=te.storage/Float32Array.BYTES_PER_ELEMENT)}r.bufferSubData(r.UNIFORM_BUFFER,F,I.__data)}}}r.bindBuffer(r.UNIFORM_BUFFER,null)}function p(S,_,R,P){let w=S.value,T=_+"_"+R;if(P[T]===void 0)return typeof w=="number"||typeof w=="boolean"?P[T]=w:P[T]=w.clone(),!0;{let C=P[T];if(typeof w=="number"||typeof w=="boolean"){if(C!==w)return P[T]=w,!0}else if(C.equals(w)===!1)return C.copy(w),!0}return!1}function x(S){let _=S.uniforms,R=0,P=16;for(let T=0,C=_.length;T<C;T++){let y=Array.isArray(_[T])?_[T]:[_[T]];for(let v=0,I=y.length;v<I;v++){let F=y[v],E=Array.isArray(F.value)?F.value:[F.value];for(let N=0,W=E.length;N<W;N++){let X=E[N],te=g(X),V=R%P;V!==0&&P-V<te.boundary&&(R+=P-V),F.__data=new Float32Array(te.storage/Float32Array.BYTES_PER_ELEMENT),F.__offset=R,R+=te.storage}}}let w=R%P;return w>0&&(R+=P-w),S.__size=R,S.__cache={},this}function g(S){let _={boundary:0,storage:0};return typeof S=="number"||typeof S=="boolean"?(_.boundary=4,_.storage=4):S.isVector2?(_.boundary=8,_.storage=8):S.isVector3||S.isColor?(_.boundary=16,_.storage=12):S.isVector4?(_.boundary=16,_.storage=16):S.isMatrix3?(_.boundary=48,_.storage=48):S.isMatrix4?(_.boundary=64,_.storage=64):S.isTexture?console.warn("THREE.WebGLRenderer: Texture samplers can not be part of an uniforms group."):console.warn("THREE.WebGLRenderer: Unsupported uniform value type.",S),_}function m(S){let _=S.target;_.removeEventListener("dispose",m);let R=s.indexOf(_.__bindingPointIndex);s.splice(R,1),r.deleteBuffer(n[_.id]),delete n[_.id],delete o[_.id]}function f(){for(let S in n)r.deleteBuffer(n[S]);s=[],n={},o={}}return{bind:c,update:l,dispose:f}}var mo=class{constructor(e={}){let{canvas:t=Wd(),context:i=null,depth:n=!0,stencil:o=!1,alpha:s=!1,antialias:a=!1,premultipliedAlpha:c=!0,preserveDrawingBuffer:l=!1,powerPreference:d="default",failIfMajorPerformanceCaveat:u=!1}=e;this.isWebGLRenderer=!0;let h;if(i!==null){if(typeof WebGLRenderingContext<"u"&&i instanceof WebGLRenderingContext)throw new Error("THREE.WebGLRenderer: WebGL 1 is not supported since r163.");h=i.getContextAttributes().alpha}else h=s;let p=new Uint32Array(4),x=new Int32Array(4),g=null,m=null,f=[],S=[];this.domElement=t,this.debug={checkShaderErrors:!0,onShaderError:null},this.autoClear=!0,this.autoClearColor=!0,this.autoClearDepth=!0,this.autoClearStencil=!0,this.sortObjects=!0,this.clippingPlanes=[],this.localClippingEnabled=!1,this._outputColorSpace=At,this._useLegacyLights=!1,this.toneMapping=ui,this.toneMappingExposure=1;let _=this,R=!1,P=0,w=0,T=null,C=-1,y=null,v=new ct,I=new ct,F=null,E=new Me(0),N=0,W=t.width,X=t.height,te=1,V=null,J=null,Q=new ct(0,0,W,X),me=new ct(0,0,W,X),Ne=!1,tt=new _r,H=!1,ee=!1,he=new Ee,ne=new D,Be={background:null,fog:null,environment:null,overrideMaterial:null,isScene:!0};function ke(){return T===null?te:1}let O=i;function ot(M,U){return t.getContext(M,U)}try{let M={alpha:!0,depth:n,stencil:o,antialias:a,premultipliedAlpha:c,preserveDrawingBuffer:l,powerPreference:d,failIfMajorPerformanceCaveat:u};if("setAttribute"in t&&t.setAttribute("data-engine",`three.js r${"164"}`),t.addEventListener("webglcontextlost",L,!1),t.addEventListener("webglcontextrestored",$,!1),t.addEventListener("webglcontextcreationerror",j,!1),O===null){let U="webgl2";if(O=ot(U,M),O===null)throw ot(U)?new Error("Error creating WebGL context with your selected attributes."):new Error("Error creating WebGL context.")}}catch(M){throw console.error("THREE.WebGLRenderer: "+M.message),M}let ve,it,Re,Ve,Ue,Ge,pt,A,b,G,q,Y,K,xe,ce,ae,Ie,re,ge,He,_e,de,De,ze;function _t(){ve=new cp(O),ve.init(),de=new Hp(O,ve),it=new Jf(O,ve,e,de),Re=new Gp(O),Ve=new up(O),Ue=new Dp,Ge=new Wp(O,ve,Re,Ue,it,de,Ve),pt=new ep(_),A=new ap(_),b=new pu(O),De=new Kf(O,b),G=new lp(O,b,Ve,De),q=new fp(O,G,b,Ve),ge=new hp(O,it,Ge),ae=new Qf(Ue),Y=new Ip(_,pt,A,ve,it,De,ae),K=new jp(_,Ue),xe=new Fp,ce=new Bp(ve),re=new Yf(_,pt,A,Re,q,h,c),Ie=new Vp(_,q,it),ze=new Xp(O,Ve,it,Re),He=new Zf(O,ve,Ve),_e=new dp(O,ve,Ve),Ve.programs=Y.programs,_.capabilities=it,_.extensions=ve,_.properties=Ue,_.renderLists=xe,_.shadowMap=Ie,_.state=Re,_.info=Ve}_t();let Le=new Hs(_,O);this.xr=Le,this.getContext=function(){return O},this.getContextAttributes=function(){return O.getContextAttributes()},this.forceContextLoss=function(){let M=ve.get("WEBGL_lose_context");M&&M.loseContext()},this.forceContextRestore=function(){let M=ve.get("WEBGL_lose_context");M&&M.restoreContext()},this.getPixelRatio=function(){return te},this.setPixelRatio=function(M){M!==void 0&&(te=M,this.setSize(W,X,!1))},this.getSize=function(M){return M.set(W,X)},this.setSize=function(M,U,z=!0){if(Le.isPresenting){console.warn("THREE.WebGLRenderer: Can't change size while VR device is presenting.");return}W=M,X=U,t.width=Math.floor(M*te),t.height=Math.floor(U*te),z===!0&&(t.style.width=M+"px",t.style.height=U+"px"),this.setViewport(0,0,M,U)},this.getDrawingBufferSize=function(M){return M.set(W*te,X*te).floor()},this.setDrawingBufferSize=function(M,U,z){W=M,X=U,te=z,t.width=Math.floor(M*z),t.height=Math.floor(U*z),this.setViewport(0,0,M,U)},this.getCurrentViewport=function(M){return M.copy(v)},this.getViewport=function(M){return M.copy(Q)},this.setViewport=function(M,U,z,B){M.isVector4?Q.set(M.x,M.y,M.z,M.w):Q.set(M,U,z,B),Re.viewport(v.copy(Q).multiplyScalar(te).round())},this.getScissor=function(M){return M.copy(me)},this.setScissor=function(M,U,z,B){M.isVector4?me.set(M.x,M.y,M.z,M.w):me.set(M,U,z,B),Re.scissor(I.copy(me).multiplyScalar(te).round())},this.getScissorTest=function(){return Ne},this.setScissorTest=function(M){Re.setScissorTest(Ne=M)},this.setOpaqueSort=function(M){V=M},this.setTransparentSort=function(M){J=M},this.getClearColor=function(M){return M.copy(re.getClearColor())},this.setClearColor=function(){re.setClearColor.apply(re,arguments)},this.getClearAlpha=function(){return re.getClearAlpha()},this.setClearAlpha=function(){re.setClearAlpha.apply(re,arguments)},this.clear=function(M=!0,U=!0,z=!0){let B=0;if(M){let k=!1;if(T!==null){let se=T.texture.format;k=se===es||se===Qo||se===Jo}if(k){let se=T.texture.type,fe=se===Ut||se===yi||se===$o||se===Fi||se===Ko||se===Zo,pe=re.getClearColor(),ye=re.getClearAlpha(),we=pe.r,Ce=pe.g,Fe=pe.b;fe?(p[0]=we,p[1]=Ce,p[2]=Fe,p[3]=ye,O.clearBufferuiv(O.COLOR,0,p)):(x[0]=we,x[1]=Ce,x[2]=Fe,x[3]=ye,O.clearBufferiv(O.COLOR,0,x))}else B|=O.COLOR_BUFFER_BIT}U&&(B|=O.DEPTH_BUFFER_BIT),z&&(B|=O.STENCIL_BUFFER_BIT,this.state.buffers.stencil.setMask(4294967295)),O.clear(B)},this.clearColor=function(){this.clear(!0,!1,!1)},this.clearDepth=function(){this.clear(!1,!0,!1)},this.clearStencil=function(){this.clear(!1,!1,!0)},this.dispose=function(){t.removeEventListener("webglcontextlost",L,!1),t.removeEventListener("webglcontextrestored",$,!1),t.removeEventListener("webglcontextcreationerror",j,!1),xe.dispose(),ce.dispose(),Ue.dispose(),pt.dispose(),A.dispose(),q.dispose(),De.dispose(),ze.dispose(),Y.dispose(),Le.dispose(),Le.removeEventListener("sessionstart",Ke),Le.removeEventListener("sessionend",Ai),Xt.stop()};function L(M){M.preventDefault(),console.log("THREE.WebGLRenderer: Context Lost."),R=!0}function $(){console.log("THREE.WebGLRenderer: Context Restored."),R=!1;let M=Ve.autoReset,U=Ie.enabled,z=Ie.autoUpdate,B=Ie.needsUpdate,k=Ie.type;_t(),Ve.autoReset=M,Ie.enabled=U,Ie.autoUpdate=z,Ie.needsUpdate=B,Ie.type=k}function j(M){console.error("THREE.WebGLRenderer: A WebGL context could not be created. Reason: ",M.statusMessage)}function oe(M){let U=M.target;U.removeEventListener("dispose",oe),le(U)}function le(M){je(M),Ue.remove(M)}function je(M){let U=Ue.get(M).programs;U!==void 0&&(U.forEach(function(z){Y.releaseProgram(z)}),M.isShaderMaterial&&Y.releaseShaderCache(M))}this.renderBufferDirect=function(M,U,z,B,k,se){U===null&&(U=Be);let fe=k.isMesh&&k.matrixWorld.determinant()<0,pe=Hm(M,U,z,B,k);Re.setMaterial(B,fe);let ye=z.index,we=1;if(B.wireframe===!0){if(ye=G.getWireframeAttribute(z),ye===void 0)return;we=2}let Ce=z.drawRange,Fe=z.attributes.position,bt=Ce.start*we,zt=(Ce.start+Ce.count)*we;se!==null&&(bt=Math.max(bt,se.start*we),zt=Math.min(zt,(se.start+se.count)*we)),ye!==null?(bt=Math.max(bt,0),zt=Math.min(zt,ye.count)):Fe!=null&&(bt=Math.max(bt,0),zt=Math.min(zt,Fe.count));let ri=zt-bt;if(ri<0||ri===1/0)return;De.setup(k,B,pe,z,ye);let ki,qe=He;if(ye!==null&&(ki=b.get(ye),qe=_e,qe.setIndex(ki)),k.isMesh)B.wireframe===!0?(Re.setLineWidth(B.wireframeLinewidth*ke()),qe.setMode(O.LINES)):qe.setMode(O.TRIANGLES);else if(k.isLine){let Te=B.linewidth;Te===void 0&&(Te=1),Re.setLineWidth(Te*ke()),k.isLineSegments?qe.setMode(O.LINES):k.isLineLoop?qe.setMode(O.LINE_LOOP):qe.setMode(O.LINE_STRIP)}else k.isPoints?qe.setMode(O.POINTS):k.isSprite&&qe.setMode(O.TRIANGLES);if(k.isBatchedMesh)k._multiDrawInstances!==null?qe.renderMultiDrawInstances(k._multiDrawStarts,k._multiDrawCounts,k._multiDrawCount,k._multiDrawInstances):qe.renderMultiDraw(k._multiDrawStarts,k._multiDrawCounts,k._multiDrawCount);else if(k.isInstancedMesh)qe.renderInstances(bt,ri,k.count);else if(z.isInstancedBufferGeometry){let Te=z._maxInstanceCount!==void 0?z._maxInstanceCount:1/0,Vn=Math.min(z.instanceCount,Te);qe.renderInstances(bt,ri,Vn)}else qe.render(bt,ri)};function st(M,U,z){M.transparent===!0&&M.side===yt&&M.forceSinglePass===!1?(M.side=ut,M.needsUpdate=!0,Po(M,U,z),M.side=ni,M.needsUpdate=!0,Po(M,U,z),M.side=yt):Po(M,U,z)}this.compile=function(M,U,z=null){z===null&&(z=M),m=ce.get(z),m.init(U),S.push(m),z.traverseVisible(function(k){k.isLight&&k.layers.test(U.layers)&&(m.pushLight(k),k.castShadow&&m.pushShadow(k))}),M!==z&&M.traverseVisible(function(k){k.isLight&&k.layers.test(U.layers)&&(m.pushLight(k),k.castShadow&&m.pushShadow(k))}),m.setupLights(_._useLegacyLights);let B=new Set;return M.traverse(function(k){let se=k.material;if(se)if(Array.isArray(se))for(let fe=0;fe<se.length;fe++){let pe=se[fe];st(pe,z,k),B.add(pe)}else st(se,z,k),B.add(se)}),S.pop(),m=null,B},this.compileAsync=function(M,U,z=null){let B=this.compile(M,U,z);return new Promise(k=>{function se(){if(B.forEach(function(fe){Ue.get(fe).currentProgram.isReady()&&B.delete(fe)}),B.size===0){k(M);return}setTimeout(se,10)}ve.get("KHR_parallel_shader_compile")!==null?se():setTimeout(se,10)})};let mt=null;function kt(M){mt&&mt(M)}function Ke(){Xt.stop()}function Ai(){Xt.start()}let Xt=new Ds;Xt.setAnimationLoop(kt),typeof self<"u"&&Xt.setContext(self),this.setAnimationLoop=function(M){mt=M,Le.setAnimationLoop(M),M===null?Xt.stop():Xt.start()},Le.addEventListener("sessionstart",Ke),Le.addEventListener("sessionend",Ai),this.render=function(M,U){if(U!==void 0&&U.isCamera!==!0){console.error("THREE.WebGLRenderer.render: camera is not an instance of THREE.Camera.");return}if(R===!0)return;M.matrixWorldAutoUpdate===!0&&M.updateMatrixWorld(),U.parent===null&&U.matrixWorldAutoUpdate===!0&&U.updateMatrixWorld(),Le.enabled===!0&&Le.isPresenting===!0&&(Le.cameraAutoUpdate===!0&&Le.updateCamera(U),U=Le.getCamera()),M.isScene===!0&&M.onBeforeRender(_,M,U,T),m=ce.get(M,S.length),m.init(U),S.push(m),he.multiplyMatrices(U.projectionMatrix,U.matrixWorldInverse),tt.setFromProjectionMatrix(he),ee=this.localClippingEnabled,H=ae.init(this.clippingPlanes,ee),g=xe.get(M,f.length),g.init(),f.push(g),Qc(M,U,0,_.sortObjects),g.finish(),_.sortObjects===!0&&g.sort(V,J);let z=Le.enabled===!1||Le.isPresenting===!1||Le.hasDepthSensing()===!1;z&&re.addToRenderList(g,M),this.info.render.frame++,H===!0&&ae.beginShadows();let B=m.state.shadowsArray;Ie.render(B,M,U),H===!0&&ae.endShadows(),this.info.autoReset===!0&&this.info.reset();let k=g.opaque,se=g.transmissive;if(m.setupLights(_._useLegacyLights),U.isArrayCamera){let fe=U.cameras;if(se.length>0)for(let pe=0,ye=fe.length;pe<ye;pe++){let we=fe[pe];tl(k,se,M,we)}z&&re.render(M);for(let pe=0,ye=fe.length;pe<ye;pe++){let we=fe[pe];el(g,M,we,we.viewport)}}else se.length>0&&tl(k,se,M,U),z&&re.render(M),el(g,M,U);T!==null&&(Ge.updateMultisampleRenderTarget(T),Ge.updateRenderTargetMipmap(T)),M.isScene===!0&&M.onAfterRender(_,M,U),De.resetDefaultState(),C=-1,y=null,S.pop(),S.length>0?(m=S[S.length-1],H===!0&&ae.setGlobalState(_.clippingPlanes,m.state.camera)):m=null,f.pop(),f.length>0?g=f[f.length-1]:g=null};function Qc(M,U,z,B){if(M.visible===!1)return;if(M.layers.test(U.layers)){if(M.isGroup)z=M.renderOrder;else if(M.isLOD)M.autoUpdate===!0&&M.update(U);else if(M.isLight)m.pushLight(M),M.castShadow&&m.pushShadow(M);else if(M.isSprite){if(!M.frustumCulled||tt.intersectsSprite(M)){B&&ne.setFromMatrixPosition(M.matrixWorld).applyMatrix4(he);let fe=q.update(M),pe=M.material;pe.visible&&g.push(M,fe,pe,z,ne.z,null)}}else if((M.isMesh||M.isLine||M.isPoints)&&(!M.frustumCulled||tt.intersectsObject(M))){let fe=q.update(M),pe=M.material;if(B&&(M.boundingSphere!==void 0?(M.boundingSphere===null&&M.computeBoundingSphere(),ne.copy(M.boundingSphere.center)):(fe.boundingSphere===null&&fe.computeBoundingSphere(),ne.copy(fe.boundingSphere.center)),ne.applyMatrix4(M.matrixWorld).applyMatrix4(he)),Array.isArray(pe)){let ye=fe.groups;for(let we=0,Ce=ye.length;we<Ce;we++){let Fe=ye[we],bt=pe[Fe.materialIndex];bt&&bt.visible&&g.push(M,fe,bt,z,ne.z,Fe)}}else pe.visible&&g.push(M,fe,pe,z,ne.z,null)}}let se=M.children;for(let fe=0,pe=se.length;fe<pe;fe++)Qc(se[fe],U,z,B)}function el(M,U,z,B){let k=M.opaque,se=M.transmissive,fe=M.transparent;m.setupLightsView(z),H===!0&&ae.setGlobalState(_.clippingPlanes,z),B&&Re.viewport(v.copy(B)),k.length>0&&Co(k,U,z),se.length>0&&Co(se,U,z),fe.length>0&&Co(fe,U,z),Re.buffers.depth.setTest(!0),Re.buffers.depth.setMask(!0),Re.buffers.color.setMask(!0),Re.setPolygonOffset(!1)}function tl(M,U,z,B){if((z.isScene===!0?z.overrideMaterial:null)!==null)return;m.state.transmissionRenderTarget[B.id]===void 0&&(m.state.transmissionRenderTarget[B.id]=new Ot(1,1,{generateMipmaps:!0,type:ve.has("EXT_color_buffer_half_float")||ve.has("EXT_color_buffer_float")?fr:Ut,minFilter:_i,samples:4,stencilBuffer:o,resolveDepthBuffer:!1,resolveStencilBuffer:!1}));let se=m.state.transmissionRenderTarget[B.id],fe=B.viewport||v;se.setSize(fe.z,fe.w);let pe=_.getRenderTarget();_.setRenderTarget(se),_.getClearColor(E),N=_.getClearAlpha(),N<1&&_.setClearColor(16777215,.5),_.clear();let ye=_.toneMapping;_.toneMapping=ui;let we=B.viewport;if(B.viewport!==void 0&&(B.viewport=void 0),m.setupLightsView(B),H===!0&&ae.setGlobalState(_.clippingPlanes,B),Co(M,z,B),Ge.updateMultisampleRenderTarget(se),Ge.updateRenderTargetMipmap(se),ve.has("WEBGL_multisampled_render_to_texture")===!1){let Ce=!1;for(let Fe=0,bt=U.length;Fe<bt;Fe++){let zt=U[Fe],ri=zt.object,ki=zt.geometry,qe=zt.material,Te=zt.group;if(qe.side===yt&&ri.layers.test(B.layers)){let Vn=qe.side;qe.side=ut,qe.needsUpdate=!0,il(ri,z,B,ki,qe,Te),qe.side=Vn,qe.needsUpdate=!0,Ce=!0}}Ce===!0&&(Ge.updateMultisampleRenderTarget(se),Ge.updateRenderTargetMipmap(se))}_.setRenderTarget(pe),_.setClearColor(E,N),we!==void 0&&(B.viewport=we),_.toneMapping=ye}function Co(M,U,z){let B=U.isScene===!0?U.overrideMaterial:null;for(let k=0,se=M.length;k<se;k++){let fe=M[k],pe=fe.object,ye=fe.geometry,we=B===null?fe.material:B,Ce=fe.group;pe.layers.test(z.layers)&&il(pe,U,z,ye,we,Ce)}}function il(M,U,z,B,k,se){M.onBeforeRender(_,U,z,B,k,se),M.modelViewMatrix.multiplyMatrices(z.matrixWorldInverse,M.matrixWorld),M.normalMatrix.getNormalMatrix(M.modelViewMatrix),k.onBeforeRender(_,U,z,B,M,se),k.transparent===!0&&k.side===yt&&k.forceSinglePass===!1?(k.side=ut,k.needsUpdate=!0,_.renderBufferDirect(z,U,B,k,M,se),k.side=ni,k.needsUpdate=!0,_.renderBufferDirect(z,U,B,k,M,se),k.side=yt):_.renderBufferDirect(z,U,B,k,M,se),M.onAfterRender(_,U,z,B,k,se)}function Po(M,U,z){U.isScene!==!0&&(U=Be);let B=Ue.get(M),k=m.state.lights,se=m.state.shadowsArray,fe=k.state.version,pe=Y.getParameters(M,k.state,se,U,z),ye=Y.getProgramCacheKey(pe),we=B.programs;B.environment=M.isMeshStandardMaterial?U.environment:null,B.fog=U.fog,B.envMap=(M.isMeshStandardMaterial?A:pt).get(M.envMap||B.environment),B.envMapRotation=B.environment!==null&&M.envMap===null?U.environmentRotation:M.envMapRotation,we===void 0&&(M.addEventListener("dispose",oe),we=new Map,B.programs=we);let Ce=we.get(ye);if(Ce!==void 0){if(B.currentProgram===Ce&&B.lightsStateVersion===fe)return nl(M,pe),Ce}else pe.uniforms=Y.getUniforms(M),M.onBuild(z,pe,_),M.onBeforeCompile(pe,_),Ce=Y.acquireProgram(pe,ye),we.set(ye,Ce),B.uniforms=pe.uniforms;let Fe=B.uniforms;return(!M.isShaderMaterial&&!M.isRawShaderMaterial||M.clipping===!0)&&(Fe.clippingPlanes=ae.uniform),nl(M,pe),B.needsLights=Xm(M),B.lightsStateVersion=fe,B.needsLights&&(Fe.ambientLightColor.value=k.state.ambient,Fe.lightProbe.value=k.state.probe,Fe.directionalLights.value=k.state.directional,Fe.directionalLightShadows.value=k.state.directionalShadow,Fe.spotLights.value=k.state.spot,Fe.spotLightShadows.value=k.state.spotShadow,Fe.rectAreaLights.value=k.state.rectArea,Fe.ltc_1.value=k.state.rectAreaLTC1,Fe.ltc_2.value=k.state.rectAreaLTC2,Fe.pointLights.value=k.state.point,Fe.pointLightShadows.value=k.state.pointShadow,Fe.hemisphereLights.value=k.state.hemi,Fe.directionalShadowMap.value=k.state.directionalShadowMap,Fe.directionalShadowMatrix.value=k.state.directionalShadowMatrix,Fe.spotShadowMap.value=k.state.spotShadowMap,Fe.spotLightMatrix.value=k.state.spotLightMatrix,Fe.spotLightMap.value=k.state.spotLightMap,Fe.pointShadowMap.value=k.state.pointShadowMap,Fe.pointShadowMatrix.value=k.state.pointShadowMatrix),B.currentProgram=Ce,B.uniformsList=null,Ce}function rl(M){if(M.uniformsList===null){let U=M.currentProgram.getUniforms();M.uniformsList=Sr.seqWithValue(U.seq,M.uniforms)}return M.uniformsList}function nl(M,U){let z=Ue.get(M);z.outputColorSpace=U.outputColorSpace,z.batching=U.batching,z.instancing=U.instancing,z.instancingColor=U.instancingColor,z.instancingMorph=U.instancingMorph,z.skinning=U.skinning,z.morphTargets=U.morphTargets,z.morphNormals=U.morphNormals,z.morphColors=U.morphColors,z.morphTargetsCount=U.morphTargetsCount,z.numClippingPlanes=U.numClippingPlanes,z.numIntersection=U.numClipIntersection,z.vertexAlphas=U.vertexAlphas,z.vertexTangents=U.vertexTangents,z.toneMapping=U.toneMapping}function Hm(M,U,z,B,k){U.isScene!==!0&&(U=Be),Ge.resetTextureUnits();let se=U.fog,fe=B.isMeshStandardMaterial?U.environment:null,pe=T===null?_.outputColorSpace:T.isXRRenderTarget===!0?T.texture.colorSpace:ei,ye=(B.isMeshStandardMaterial?A:pt).get(B.envMap||fe),we=B.vertexColors===!0&&!!z.attributes.color&&z.attributes.color.itemSize===4,Ce=!!z.attributes.tangent&&(!!B.normalMap||B.anisotropy>0),Fe=!!z.morphAttributes.position,bt=!!z.morphAttributes.normal,zt=!!z.morphAttributes.color,ri=ui;B.toneMapped&&(T===null||T.isXRRenderTarget===!0)&&(ri=_.toneMapping);let ki=z.morphAttributes.position||z.morphAttributes.normal||z.morphAttributes.color,qe=ki!==void 0?ki.length:0,Te=Ue.get(B),Vn=m.state.lights;if(H===!0&&(ee===!0||M!==y)){let li=M===y&&B.id===C;ae.setState(B,M,li)}let lt=!1;B.version===Te.__version?(Te.needsLights&&Te.lightsStateVersion!==Vn.state.version||Te.outputColorSpace!==pe||k.isBatchedMesh&&Te.batching===!1||!k.isBatchedMesh&&Te.batching===!0||k.isInstancedMesh&&Te.instancing===!1||!k.isInstancedMesh&&Te.instancing===!0||k.isSkinnedMesh&&Te.skinning===!1||!k.isSkinnedMesh&&Te.skinning===!0||k.isInstancedMesh&&Te.instancingColor===!0&&k.instanceColor===null||k.isInstancedMesh&&Te.instancingColor===!1&&k.instanceColor!==null||k.isInstancedMesh&&Te.instancingMorph===!0&&k.morphTexture===null||k.isInstancedMesh&&Te.instancingMorph===!1&&k.morphTexture!==null||Te.envMap!==ye||B.fog===!0&&Te.fog!==se||Te.numClippingPlanes!==void 0&&(Te.numClippingPlanes!==ae.numPlanes||Te.numIntersection!==ae.numIntersection)||Te.vertexAlphas!==we||Te.vertexTangents!==Ce||Te.morphTargets!==Fe||Te.morphNormals!==bt||Te.morphColors!==zt||Te.toneMapping!==ri||Te.morphTargetsCount!==qe)&&(lt=!0):(lt=!0,Te.__version=B.version);let Tr=Te.currentProgram;lt===!0&&(Tr=Po(B,U,k));let ol=!1,Gn=!1,sa=!1,Vt=Tr.getUniforms(),ar=Te.uniforms;if(Re.useProgram(Tr.program)&&(ol=!0,Gn=!0,sa=!0),B.id!==C&&(C=B.id,Gn=!0),ol||y!==M){Vt.setValue(O,"projectionMatrix",M.projectionMatrix),Vt.setValue(O,"viewMatrix",M.matrixWorldInverse);let li=Vt.map.cameraPosition;li!==void 0&&li.setValue(O,ne.setFromMatrixPosition(M.matrixWorld)),it.logarithmicDepthBuffer&&Vt.setValue(O,"logDepthBufFC",2/(Math.log(M.far+1)/Math.LN2)),(B.isMeshPhongMaterial||B.isMeshToonMaterial||B.isMeshLambertMaterial||B.isMeshBasicMaterial||B.isMeshStandardMaterial||B.isShaderMaterial)&&Vt.setValue(O,"isOrthographic",M.isOrthographicCamera===!0),y!==M&&(y=M,Gn=!0,sa=!0)}if(k.isSkinnedMesh){Vt.setOptional(O,k,"bindMatrix"),Vt.setOptional(O,k,"bindMatrixInverse");let li=k.skeleton;li&&(li.boneTexture===null&&li.computeBoneTexture(),Vt.setValue(O,"boneTexture",li.boneTexture,Ge))}k.isBatchedMesh&&(Vt.setOptional(O,k,"batchingTexture"),Vt.setValue(O,"batchingTexture",k._matricesTexture,Ge));let aa=z.morphAttributes;if((aa.position!==void 0||aa.normal!==void 0||aa.color!==void 0)&&ge.update(k,z,Tr),(Gn||Te.receiveShadow!==k.receiveShadow)&&(Te.receiveShadow=k.receiveShadow,Vt.setValue(O,"receiveShadow",k.receiveShadow)),B.isMeshGouraudMaterial&&B.envMap!==null&&(ar.envMap.value=ye,ar.flipEnvMap.value=ye.isCubeTexture&&ye.isRenderTargetTexture===!1?-1:1),B.isMeshStandardMaterial&&B.envMap===null&&U.environment!==null&&(ar.envMapIntensity.value=U.environmentIntensity),Gn&&(Vt.setValue(O,"toneMappingExposure",_.toneMappingExposure),Te.needsLights&&jm(ar,sa),se&&B.fog===!0&&K.refreshFogUniforms(ar,se),K.refreshMaterialUniforms(ar,B,te,X,m.state.transmissionRenderTarget[M.id]),Sr.upload(O,rl(Te),ar,Ge)),B.isShaderMaterial&&B.uniformsNeedUpdate===!0&&(Sr.upload(O,rl(Te),ar,Ge),B.uniformsNeedUpdate=!1),B.isSpriteMaterial&&Vt.setValue(O,"center",k.center),Vt.setValue(O,"modelViewMatrix",k.modelViewMatrix),Vt.setValue(O,"normalMatrix",k.normalMatrix),Vt.setValue(O,"modelMatrix",k.matrixWorld),B.isShaderMaterial||B.isRawShaderMaterial){let li=B.uniformsGroups;for(let ca=0,qm=li.length;ca<qm;ca++){let sl=li[ca];ze.update(sl,Tr),ze.bind(sl,Tr)}}return Tr}function jm(M,U){M.ambientLightColor.needsUpdate=U,M.lightProbe.needsUpdate=U,M.directionalLights.needsUpdate=U,M.directionalLightShadows.needsUpdate=U,M.pointLights.needsUpdate=U,M.pointLightShadows.needsUpdate=U,M.spotLights.needsUpdate=U,M.spotLightShadows.needsUpdate=U,M.rectAreaLights.needsUpdate=U,M.hemisphereLights.needsUpdate=U}function Xm(M){return M.isMeshLambertMaterial||M.isMeshToonMaterial||M.isMeshPhongMaterial||M.isMeshStandardMaterial||M.isShadowMaterial||M.isShaderMaterial&&M.lights===!0}this.getActiveCubeFace=function(){return P},this.getActiveMipmapLevel=function(){return w},this.getRenderTarget=function(){return T},this.setRenderTargetTextures=function(M,U,z){Ue.get(M.texture).__webglTexture=U,Ue.get(M.depthTexture).__webglTexture=z;let B=Ue.get(M);B.__hasExternalTextures=!0,B.__autoAllocateDepthBuffer=z===void 0,B.__autoAllocateDepthBuffer||ve.has("WEBGL_multisampled_render_to_texture")===!0&&(console.warn("THREE.WebGLRenderer: Render-to-texture extension was disabled because an external texture was provided"),B.__useRenderToTexture=!1)},this.setRenderTargetFramebuffer=function(M,U){let z=Ue.get(M);z.__webglFramebuffer=U,z.__useDefaultFramebuffer=U===void 0},this.setRenderTarget=function(M,U=0,z=0){T=M,P=U,w=z;let B=!0,k=null,se=!1,fe=!1;if(M){let ye=Ue.get(M);ye.__useDefaultFramebuffer!==void 0?(Re.bindFramebuffer(O.FRAMEBUFFER,null),B=!1):ye.__webglFramebuffer===void 0?Ge.setupRenderTarget(M):ye.__hasExternalTextures&&Ge.rebindTextures(M,Ue.get(M.texture).__webglTexture,Ue.get(M.depthTexture).__webglTexture);let we=M.texture;(we.isData3DTexture||we.isDataArrayTexture||we.isCompressedArrayTexture)&&(fe=!0);let Ce=Ue.get(M).__webglFramebuffer;M.isWebGLCubeRenderTarget?(Array.isArray(Ce[U])?k=Ce[U][z]:k=Ce[U],se=!0):M.samples>0&&Ge.useMultisampledRTT(M)===!1?k=Ue.get(M).__webglMultisampledFramebuffer:Array.isArray(Ce)?k=Ce[z]:k=Ce,v.copy(M.viewport),I.copy(M.scissor),F=M.scissorTest}else v.copy(Q).multiplyScalar(te).floor(),I.copy(me).multiplyScalar(te).floor(),F=Ne;if(Re.bindFramebuffer(O.FRAMEBUFFER,k)&&B&&Re.drawBuffers(M,k),Re.viewport(v),Re.scissor(I),Re.setScissorTest(F),se){let ye=Ue.get(M.texture);O.framebufferTexture2D(O.FRAMEBUFFER,O.COLOR_ATTACHMENT0,O.TEXTURE_CUBE_MAP_POSITIVE_X+U,ye.__webglTexture,z)}else if(fe){let ye=Ue.get(M.texture),we=U||0;O.framebufferTextureLayer(O.FRAMEBUFFER,O.COLOR_ATTACHMENT0,ye.__webglTexture,z||0,we)}C=-1},this.readRenderTargetPixels=function(M,U,z,B,k,se,fe){if(!(M&&M.isWebGLRenderTarget)){console.error("THREE.WebGLRenderer.readRenderTargetPixels: renderTarget is not THREE.WebGLRenderTarget.");return}let pe=Ue.get(M).__webglFramebuffer;if(M.isWebGLCubeRenderTarget&&fe!==void 0&&(pe=pe[fe]),pe){Re.bindFramebuffer(O.FRAMEBUFFER,pe);try{let ye=M.texture,we=ye.format,Ce=ye.type;if(!it.textureFormatReadable(we)){console.error("THREE.WebGLRenderer.readRenderTargetPixels: renderTarget is not in RGBA or implementation defined format.");return}if(!it.textureTypeReadable(Ce)){console.error("THREE.WebGLRenderer.readRenderTargetPixels: renderTarget is not in UnsignedByteType or implementation defined type.");return}U>=0&&U<=M.width-B&&z>=0&&z<=M.height-k&&O.readPixels(U,z,B,k,de.convert(we),de.convert(Ce),se)}finally{let ye=T!==null?Ue.get(T).__webglFramebuffer:null;Re.bindFramebuffer(O.FRAMEBUFFER,ye)}}},this.copyFramebufferToTexture=function(M,U,z=0){let B=Math.pow(2,-z),k=Math.floor(U.image.width*B),se=Math.floor(U.image.height*B);Ge.setTexture2D(U,0),O.copyTexSubImage2D(O.TEXTURE_2D,z,0,0,M.x,M.y,k,se),Re.unbindTexture()},this.copyTextureToTexture=function(M,U,z,B=0){let k=U.image.width,se=U.image.height,fe=de.convert(z.format),pe=de.convert(z.type);Ge.setTexture2D(z,0),O.pixelStorei(O.UNPACK_FLIP_Y_WEBGL,z.flipY),O.pixelStorei(O.UNPACK_PREMULTIPLY_ALPHA_WEBGL,z.premultiplyAlpha),O.pixelStorei(O.UNPACK_ALIGNMENT,z.unpackAlignment),U.isDataTexture?O.texSubImage2D(O.TEXTURE_2D,B,M.x,M.y,k,se,fe,pe,U.image.data):U.isCompressedTexture?O.compressedTexSubImage2D(O.TEXTURE_2D,B,M.x,M.y,U.mipmaps[0].width,U.mipmaps[0].height,fe,U.mipmaps[0].data):O.texSubImage2D(O.TEXTURE_2D,B,M.x,M.y,fe,pe,U.image),B===0&&z.generateMipmaps&&O.generateMipmap(O.TEXTURE_2D),Re.unbindTexture()},this.copyTextureToTexture3D=function(M,U,z,B,k=0){let se=M.max.x-M.min.x,fe=M.max.y-M.min.y,pe=M.max.z-M.min.z,ye=de.convert(B.format),we=de.convert(B.type),Ce;if(B.isData3DTexture)Ge.setTexture3D(B,0),Ce=O.TEXTURE_3D;else if(B.isDataArrayTexture||B.isCompressedArrayTexture)Ge.setTexture2DArray(B,0),Ce=O.TEXTURE_2D_ARRAY;else{console.warn("THREE.WebGLRenderer.copyTextureToTexture3D: only supports THREE.DataTexture3D and THREE.DataTexture2DArray.");return}O.pixelStorei(O.UNPACK_FLIP_Y_WEBGL,B.flipY),O.pixelStorei(O.UNPACK_PREMULTIPLY_ALPHA_WEBGL,B.premultiplyAlpha),O.pixelStorei(O.UNPACK_ALIGNMENT,B.unpackAlignment);let Fe=O.getParameter(O.UNPACK_ROW_LENGTH),bt=O.getParameter(O.UNPACK_IMAGE_HEIGHT),zt=O.getParameter(O.UNPACK_SKIP_PIXELS),ri=O.getParameter(O.UNPACK_SKIP_ROWS),ki=O.getParameter(O.UNPACK_SKIP_IMAGES),qe=z.isCompressedTexture?z.mipmaps[k]:z.image;O.pixelStorei(O.UNPACK_ROW_LENGTH,qe.width),O.pixelStorei(O.UNPACK_IMAGE_HEIGHT,qe.height),O.pixelStorei(O.UNPACK_SKIP_PIXELS,M.min.x),O.pixelStorei(O.UNPACK_SKIP_ROWS,M.min.y),O.pixelStorei(O.UNPACK_SKIP_IMAGES,M.min.z),z.isDataTexture||z.isData3DTexture?O.texSubImage3D(Ce,k,U.x,U.y,U.z,se,fe,pe,ye,we,qe.data):B.isCompressedArrayTexture?O.compressedTexSubImage3D(Ce,k,U.x,U.y,U.z,se,fe,pe,ye,qe.data):O.texSubImage3D(Ce,k,U.x,U.y,U.z,se,fe,pe,ye,we,qe),O.pixelStorei(O.UNPACK_ROW_LENGTH,Fe),O.pixelStorei(O.UNPACK_IMAGE_HEIGHT,bt),O.pixelStorei(O.UNPACK_SKIP_PIXELS,zt),O.pixelStorei(O.UNPACK_SKIP_ROWS,ri),O.pixelStorei(O.UNPACK_SKIP_IMAGES,ki),k===0&&B.generateMipmaps&&O.generateMipmap(Ce),Re.unbindTexture()},this.initTexture=function(M){M.isCubeTexture?Ge.setTextureCube(M,0):M.isData3DTexture?Ge.setTexture3D(M,0):M.isDataArrayTexture||M.isCompressedArrayTexture?Ge.setTexture2DArray(M,0):Ge.setTexture2D(M,0),Re.unbindTexture()},this.resetState=function(){P=0,w=0,T=null,Re.reset(),De.reset()},typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}get coordinateSystem(){return ti}get outputColorSpace(){return this._outputColorSpace}set outputColorSpace(e){this._outputColorSpace=e;let t=this.getContext();t.drawingBufferColorSpace=e===Qr?"display-p3":"srgb",t.unpackColorSpace=Oe.workingColorSpace===Ir?"display-p3":"srgb"}get useLegacyLights(){return console.warn("THREE.WebGLRenderer: The property .useLegacyLights has been deprecated. Migrate your lighting according to the following guide: https://discourse.threejs.org/t/updates-to-lighting-in-three-js-r155/53733."),this._useLegacyLights}set useLegacyLights(e){console.warn("THREE.WebGLRenderer: The property .useLegacyLights has been deprecated. Migrate your lighting according to the following guide: https://discourse.threejs.org/t/updates-to-lighting-in-three-js-r155/53733."),this._useLegacyLights=e}};var go=class{#e=new Map;install(e){let t=new Set;for(let i of e){if(t.has(i.key))return Object.freeze({status:"rejected",key:i.debugKey,message:`resource ${i.debugKey} occurs more than once in the installation`});if(t.add(i.key),this.#e.has(i.key))return Object.freeze({status:"rejected",key:i.debugKey,message:`resource ${i.debugKey} is already installed`})}for(let i of e)this.#e.set(i.key,{value:i.value,references:0,retired:!1});return Object.freeze({status:"accepted"})}acquire(e){let t=this.#e.get(e);if(!t||t.retired)return null;t.references+=1;let i=!1;return Object.freeze({value:t.value,release:()=>{i||(i=!0,this.#i(e,t))}})}retire(e){let t=this.#e.get(e);return!t||t.retired?!1:(t.retired=!0,t.references===0&&(this.#e.delete(e),t.value.dispose()),!0)}referenceCount(e){return this.#e.get(e)?.references??0}has(e){let t=this.#e.get(e);return t!==void 0&&!t.retired}disposeAll(){let e=[...this.#e.values()];this.#e.clear(),rt("resource registry disposal",e.map(t=>()=>t.value.dispose()))}#i(e,t){let i=this.#e.get(e);if(i!==t)throw new Error("resource lease no longer names its registry entry");if(i.references-=1,i.references<0)throw new Error("resource reference count became negative");i.references===0&&i.retired&&(this.#e.delete(e),i.value.dispose())}};function Wr(r,e,t){return JSON.stringify(["font",r,e,t])}function xo(r,e,t,i){return JSON.stringify(["image",r,e,t,i])}var Cn=class{#e=new go;#i=new go;#n=new Set;#t=new Map;#o=new Map;#r=new Set;#s=new Set;#a;#c=!1;constructor(e=Mv()){this.#a=e}installResources(e){if(this.#c)return An("invalid_schema","$","renderer resources have been disposed");let t;try{t=vv(e)}catch(a){return a instanceof Ze?An(a.code,a.path,a.message):An("invalid_schema","$",a instanceof Error?a.message:String(a))}if(this.#n.has(t.generationId))return An("duplicate_identity","$.generationId",`generation ${t.generationId} resources are already installed`,t.generationId);let i=[],n=[];try{for(let a of t.fonts){let c=this.#a.createFontFace(a.family,a.bytes.slice().buffer),l={family:a.family,face:c,state:"loading",failure:null,dispose:()=>{l.state!=="retired"&&(l.state="retired",l.failure=null,this.#a.deleteFont(c))}};i.push({key:a.key,value:l})}for(let a of t.imageFrames)n.push({key:a.key,value:_v(a)})}catch(a){return wv(i,n),An("invalid_schema","$",`resource allocation failed: ${a instanceof Error?a.message:String(a)}`,t.generationId)}let o=this.#e.install(i.map(({key:a,value:c})=>({key:a,debugKey:a,value:c}))),s=this.#i.install(n.map(({key:a,value:c})=>({key:a,debugKey:a,value:c})));if(o.status==="rejected"||s.status==="rejected"){for(let c of i)this.#e.retire(c.key);for(let c of n)this.#i.retire(c.key);let a=o.status==="rejected"?o.message:s.status==="rejected"?s.message:"resource installation failed";return An("duplicate_identity","$",a,t.generationId)}this.#n.add(t.generationId),this.#t.set(t.generationId,Object.freeze(t.fonts.map(a=>a.key))),this.#o.set(t.generationId,Object.freeze(t.imageFrames.map(a=>a.key)));for(let a of t.fonts)this.#r.add(a.key);for(let a of t.imageFrames)this.#s.add($p(a.key));for(let{value:a}of i)a.face.load().then(c=>{if(a.state==="loading")try{this.#a.addFont(c),a.state==="loading"&&(a.state="ready")}catch(l){if(a.state!=="loading")return;a.state="failed",a.failure=l instanceof Error?l.message:String(l)}},c=>{a.state==="loading"&&(a.state="failed",a.failure=c instanceof Error?c.message:String(c))});return Object.freeze({status:"accepted"})}acquireRasterImage(e,t){let i=this.#i.acquire(e);return i?{status:"ready",lease:Object.freeze({texture:i.value[t],release:i.release})}:{status:"failed",message:`unknown installed image frame ${e}`}}hasFontResource(e){return this.#e.has(e)}hasGeneration(e){return this.#n.has(e)}hasImageBinding(e,t,i){return this.#s.has(JSON.stringify(["image",e,t,i]))}hasImageFrameResource(e){return this.#i.has(e)}measureShapedText(e,t,i){let n=this.#e.acquire(e);if(!n)return{status:"failed",message:`unknown installed font ${e}`};if(n.value.state==="loading")return n.release(),{status:"pending"};if(n.value.state==="failed"){let o=n.value.failure??"font loading failed";return n.release(),{status:"failed",message:o}}try{let o=Yp(this.#a,n.value,t,i);return{status:"ready",lease:Object.freeze({...o,release:n.release})}}catch(o){return n.release(),{status:"failed",message:o instanceof Error?o.message:String(o)}}}acquireShapedText(e){let t=this.#e.acquire(e.resourceKey);if(!t)return{status:"failed",message:`unknown installed font ${e.resourceKey}`};if(t.value.state==="loading")return t.release(),{status:"pending"};if(t.value.state==="failed"){let i=t.value.failure??"font loading failed";return t.release(),{status:"failed",message:i}}try{return{status:"ready",lease:yv(this.#a,t,e)}}catch(i){return t.release(),{status:"failed",message:i instanceof Error?i.message:String(i)}}}retireGeneration(e){if(!this.#n.delete(e))return!1;for(let t of this.#t.get(e)??[])this.#e.retire(t),this.#r.delete(t);for(let t of this.#o.get(e)??[])this.#i.retire(t),this.#s.delete($p(t));return this.#t.delete(e),this.#o.delete(e),!0}retireResource(e){return this.#e.retire(e)||this.#i.retire(e)}dispose(){this.#c||(this.#c=!0,rt("installed renderer resource disposal",[()=>this.#e.disposeAll(),()=>this.#i.disposeAll(),()=>{this.#n.clear(),this.#t.clear(),this.#o.clear(),this.#r.clear(),this.#s.clear()}]))}};function vv(r){let e=Pi(r,"$",["version","generationId","fonts","images"]);dl(e.version,1,"$.version");let i=Gt(e.generationId,"$.generationId").toString(),n=[],o=new Set;for(let[l,d]of Xn(e.fonts,"$.fonts").entries()){let u=`$.fonts[${l}]`,h=Pi(d,u,["id","revision","bytes"]),p=qn(h.id,`${u}.id`),x=qn(h.revision,`${u}.revision`),g=Wr(i,p,x);if(o.has(g))throw new Ze("invalid_schema",u,`duplicate font identity ${g}`);o.add(g);let m=qp(h.bytes,`${u}.bytes`);if(m.length===0)throw new Ze("invalid_schema",`${u}.bytes`,"font bytes must not be empty");n.push({key:g,bytes:m,family:`PuzzleRendererFont_${Tv(g)}`})}let s=[],a=new Set,c=new Set;for(let[l,d]of Xn(e.images,"$.images").entries()){let u=`$.images[${l}]`,h=Pi(d,u,["id","revision","width","height","frames"]),p=qn(h.id,`${u}.id`),x=qn(h.revision,`${u}.revision`),g=JSON.stringify([i,p,x]);if(c.has(g))throw new Ze("invalid_schema",u,`duplicate image identity ${g}`);c.add(g);let m=js(h.width,`${u}.width`,65535),f=js(h.height,`${u}.height`,65535);if(m===0||f===0)throw new Ze("invalid_schema",u,"image dimensions must be non-zero");let S=Xn(h.frames,`${u}.frames`);if(S.length===0)throw new Ze("invalid_schema",`${u}.frames`,"image frames must not be empty");for(let[_,R]of S.entries()){let P=`${u}.frames[${_}]`,w=Pi(R,P,["id","rgba8Srgb"]),T=js(w.id,`${P}.id`,4294967295),C=xo(i,p,x,T);if(a.has(C))throw new Ze("invalid_schema",P,`duplicate image frame identity ${C}`);a.add(C);let y=qp(w.rgba8Srgb,`${P}.rgba8Srgb`),v=m*f*4;if(y.length!==v)throw new Ze("invalid_schema",`${P}.rgba8Srgb`,`image frame requires exactly ${v} RGBA8 bytes`);s.push({key:C,width:m,height:f,rgba8Srgb:y})}}return{generationId:i,fonts:n,imageFrames:s}}function _v(r){let e=r.rgba8Srgb.slice(),t=new vn(e,r.width,r.height,Mt,Ut);Ac(t,nt);let i=t.clone();return i.image={data:r.rgba8Srgb.slice(),width:r.width,height:r.height},Ac(i,ht),{pixelated:t,smooth:i,dispose:()=>{t.dispose(),i.dispose()}}}function Ac(r,e){r.colorSpace=At,r.flipY=!0,r.generateMipmaps=!1,r.unpackAlignment=1,r.magFilter=e,r.minFilter=e,r.wrapS=Yt,r.wrapT=Yt,r.needsUpdate=!0}function yv(r,e,t){let i=t.units==="cell"?128:1,n=t.fontSize*i,o=t.value.split(`
`),s=o.map(_=>Yp(r,e.value,_,n)),a=t.lineHeight*i;if(!Number.isFinite(a)||a<=0)throw new Error("resolved text line height must be finite and positive");let c=Math.max(a,...s.map(_=>_.height)),l=Math.max(1,...s.map(_=>_.width)),d=Math.max(1,Math.ceil(c*o.length)),u=`${n}px "${e.value.family}"`,p=r.createCanvas(l,d).getContext("2d",{willReadFrequently:!0});if(!p)throw new Error("OffscreenCanvas 2D context is unavailable for installed-font rasterization");p.clearRect(0,0,l,d),p.font=u,p.textAlign="left",p.textBaseline="alphabetic",p.fillStyle="rgba(255, 255, 255, 1)";for(let _=0;_<o.length;_+=1){let R=s[_],P=_*c+(c-R.height)/2;p.fillText(o[_],1+R.actualLeft,P+1+R.ascent)}let x=p.getImageData(0,0,l,d).data.slice(),g=new vn(x,l,d,Mt,Ut);Ac(g,ht);let m=!1,f=t.units==="cell"?Sv(t.align,l/i,d/i):Rv(t.align,l,d),S=t.units==="cell"&&t.overflow==="clip"?bv(f):{destination:f,uv:Object.freeze({x:0,y:0,width:1,height:1})};return Object.freeze({texture:g,destination:S.destination,uv:S.uv,release:()=>{m||(m=!0,g.dispose(),e.release())}})}function Sv(r,e,t){let i=r.endsWith("left")||r==="left"?0:r.endsWith("right")||r==="right"?1-e:(1-e)/2,n=r.startsWith("top")||r==="top"?0:r.startsWith("bottom")||r==="bottom"?1-t:(1-t)/2;return Object.freeze({x:i,y:n,width:e,height:t})}function bv(r){let e=Math.max(0,r.x),t=Math.max(0,r.y),i=Math.min(1,r.x+r.width),n=Math.min(1,r.y+r.height);return i<=e||n<=t?{destination:Object.freeze({x:0,y:0,width:0,height:0}),uv:Object.freeze({x:0,y:0,width:0,height:0})}:{destination:Object.freeze({x:e,y:t,width:i-e,height:n-t}),uv:Object.freeze({x:(e-r.x)/r.width,y:(t-r.y)/r.height,width:(i-e)/r.width,height:(n-t)/r.height})}}function Yp(r,e,t,i){let o=r.createCanvas(1,1).getContext("2d",{willReadFrequently:!0});if(!o)throw new Error("OffscreenCanvas 2D context is unavailable for installed-font shaping");o.font=`${i}px "${e.family}"`,o.textAlign="left",o.textBaseline="alphabetic";let s=o.measureText(t);if(![s.actualBoundingBoxLeft,s.actualBoundingBoxRight,s.actualBoundingBoxAscent,s.actualBoundingBoxDescent].every(Number.isFinite))throw new Error("browser text shaper did not return exact glyph bounds");let c=Math.ceil(s.actualBoundingBoxLeft),l=Math.ceil(s.actualBoundingBoxRight),d=Math.ceil(s.actualBoundingBoxAscent),u=Math.ceil(s.actualBoundingBoxDescent);return Object.freeze({width:Math.max(1,c+l+2),height:Math.max(1,d+u+2),actualLeft:c,actualRight:l,ascent:d,descent:u})}function Rv(r,e,t){let i=r.endsWith("left")||r==="left"?0:r.endsWith("right")||r==="right"?-e:-e/2,n=r.startsWith("top")||r==="top"?0:r.startsWith("bottom")||r==="bottom"?-t:-t/2;return Object.freeze({x:i,y:n,width:e,height:t})}function qp(r,e){let t=Xn(r,e),i=new Uint8Array(t.length);for(let n=0;n<t.length;n+=1)i[n]=js(t[n],`${e}[${n}]`,255);return i}function js(r,e,t){if(!Number.isInteger(r)||typeof r!="number"||r<0||r>t)throw new Ze("invalid_schema",e,`${e} must be an unsigned integer no greater than ${t}`);return r}function An(r,e,t,i){let n={code:r,path:e,message:t};return i!==void 0&&(n.generationId=i),Object.freeze({status:"rejected",rejection:Object.freeze(n)})}function Mv(){let r=globalThis;if(typeof globalThis.FontFace!="function"||!r.fonts)throw new Error("Worker FontFaceSet is unavailable");return{createFontFace:(e,t)=>new FontFace(e,t,{display:"block",unicodeRange:"U+0-10FFFF"}),addFont:e=>{r.fonts.add(e)},deleteFont:e=>{r.fonts.delete(e)},createCanvas:(e,t)=>new OffscreenCanvas(e,t)}}function wv(r,e){for(let t of r)t.value.dispose();for(let t of e)t.value.dispose()}function Tv(r){let e=2166136261;for(let t=0;t<r.length;t+=1)e^=r.charCodeAt(t),e=Math.imul(e,16777619);return(e>>>0).toString(16).padStart(8,"0")}function $p(r){let[e,t,i,n]=JSON.parse(r);return JSON.stringify([e,t,i,n])}var Ln=class r{#e=new Map;#i=new Map;#n=null;#t=0n;get revision(){return this.#t}fork(){let e=new r;for(let[t,i]of this.#e)e.#e.set(t,{...i});for(let[t,i]of this.#i)e.#i.set(t,{...i});return e.#n=this.#n===null?null:{...this.#n},e.#t=this.#t,e}reset(){this.#e.clear(),this.#i.clear(),this.#n=null,this.#t=0n}activateLayout(e){let t=new Set(e.scrollableRegions.map(n=>n.key)),i=new Set(e.cameraRegions.map(n=>n.key));for(let n of this.#e.keys())t.has(n)||this.#e.delete(n);for(let n of this.#i.keys())i.has(n)||this.#i.delete(n);this.#n&&!i.has(this.#n.viewport)&&(this.#n=null)}scrollOffset(e,t,i){let n=this.#e.get(e)??{x:0,y:0},o={x:Math.min(Math.max(n.x,0),t),y:Math.min(Math.max(n.y,0),i)};return this.#e.set(e,o),o}reveal(e,t,i,n,o){let s=this.scrollOffset(e,t,i),a=s.x,c=s.y;o.x-a<n.x&&(a=o.x-n.x),o.x+o.width-a>n.x+n.width&&(a=o.x+o.width-n.x-n.width),o.y-c<n.y&&(c=o.y-n.y),o.y+o.height-c>n.y+n.height&&(c=o.y+o.height-n.y-n.height),this.#e.set(e,{x:Math.min(Math.max(a,0),t),y:Math.min(Math.max(c,0),i)})}camera(e){return this.#i.get(e)??Object.freeze({yaw:0,pitch:0,zoom:1})}apply(e,t){let i=!1;switch(t.kind){case"wheel":{let n=t.point,o=n.x,s=n.y,a=e.scrollableRegions.find(l=>Pc(l.rect,o,s));if(a){let l=t.delta,d=t.unit==="line"?a.lineHeight:1,u=this.scrollOffset(a.key,a.maximumX,a.maximumY),h={x:Math.min(Math.max(u.x+l.x*d,0),a.maximumX),y:Math.min(Math.max(u.y+l.y*d,0),a.maximumY)};if(h.x!==u.x||h.y!==u.y){this.#e.set(a.key,h),i=!0;break}}let c=e.cameraRegions.find(l=>l.interactiveZoom&&Pc(l.rect,o,s));if(c){let l=t.delta,d=this.#o(c.key),u=Math.min(Math.max(d.zoom*Math.exp(-l.y*.001),.1),10);u!==d.zoom&&(d.zoom=u,i=!0)}break}case"press":{let n=t.point,o=e.cameraRegions.find(s=>s.interactiveLook&&Pc(s.rect,n.x,n.y));o&&(this.#n={pointer:String(t.pointer),viewport:o.key});break}case"move":if(this.#n?.pointer===String(t.pointer)){let n=this.#o(this.#n.viewport),o=t.delta;n.yaw+=o.x*.25,n.pitch=Math.min(Math.max(n.pitch-o.y*.25,-89),89),i=o.x!==0||o.y!==0}break;case"release":case"cancel":this.#n?.pointer===String(t.pointer)&&(this.#n=null);break}return i&&(this.#t=this.#t+1n),{ok:!0,changed:i,interactionRevision:this.#t}}#o(e){let t=this.#i.get(e);return t||(t={yaw:0,pitch:0,zoom:1},this.#i.set(e,t)),t}},Hr=class{#e;#i;constructor(e,t){this.#e=e,this.#i=t}project(e,t,i){return this.#n(e,t,i,!1)}projectRevisionRoot(e,t,i){return this.#n(e,t,i,!0)}#n(e,t,i,n){if(i.interactionRevision!==this.#i.revision)return Pn("wrong_generation","$.interactionRevision","sample does not name current renderer interaction state");let o=i.extent;if(o.width<=0||o.height<=0)return Pn("invalid_schema","$.extent","candidate extent must be non-zero");let a=i.configuration.theme,c=a.uiSkin,l=a.font,d=this.#t(e,l,"$.configuration.theme.font");if(d.status!=="ready")return d;let u=new Map(i.surface.nodes.map(y=>[Pe(y.id),y])),h=new Map(i.viewports.map(y=>[Pe(y.id),y])),p=Bv(a,o),x=kv(a,o,p),g=new Map,m=new Set;try{for(let y of i.surface.components)y.visibility==="visible"&&this.#o(y.rootNode,u,g,m,a,d.key,p)}catch(y){return y instanceof In?{status:"pending"}:Pn("invalid_schema","$.surface",y instanceof Error?y.message:String(y))}let f=[],S=[],_=[],R=[],P=[...i.surface.components].filter(y=>y.visibility==="visible").sort(Vv),w=0;try{for(let y of P){let v=[],I=[],F=[],E=zv(y,x,c,p);y.modalSurface===!0&&v.push(Xs(`modal:${String(y.id)}`,BigInt(v.length),E,c.modal,x,p)),this.#c({nodeId:y.rootNode,nodes:u,measurements:g,viewportTargets:h,theme:a,fontKey:d.key,generationId:e,rect:E,clip:E,scale:p,underlay:v,overlay:I,viewports:F,hitRegions:S,scrollableRegions:_,cameraRegions:R,retainClippedResources:n}),this.#a({nodeId:y.rootNode,nodes:u,measurements:g,viewportTargets:h,theme:a,fontKey:d.key,generationId:e,rect:E,clip:E,scale:p,underlay:v,overlay:I,viewports:F,hitRegions:S,scrollableRegions:_,cameraRegions:R,retainClippedResources:n}),v.length>0&&f.push(Kp(`ui:${w++}:underlay`,o,v,qs(a.renderTreatment))),f.push(...F),I.length>0&&f.push(Kp(`ui:${w++}:overlay`,o,I,qs(a.renderTreatment)))}}catch(y){return y instanceof In?{status:"pending"}:Pn("invalid_schema","$.surface",y instanceof Error?y.message:String(y))}let T=Object.freeze({regions:Object.freeze(S.reverse())}),C=Object.freeze({hitMap:T,scrollableRegions:Object.freeze([..._].reverse()),cameraRegions:Object.freeze([...R].reverse())});return{status:"ready",candidate:Object.freeze({generationId:BigInt(e),requestId:t,extent:Object.freeze({width:o.width,height:o.height}),interactionRevision:i.interactionRevision,background:c.canvas,backdrop:Pv(a.backdropTreatment),viewports:Object.freeze(f),hitMap:T}),layout:C}}#t(e,t,i){if(typeof t.id!="string"||typeof t.revision!="string")return Pn("unknown_resource",i,"resolved font must name exact installed id and revision");let n=Wr(e,t.id,t.revision);return this.#e.hasFontResource(n)?{status:"ready",key:n}:Pn("unknown_resource",i,`font ${t.id}@${t.revision} is not installed`)}#o(e,t,i,n,o,s,a){let c=Pe(e),l=i.get(c);if(l)return l;if(n.has(c))throw new Error(`scene node cycle at ${c}`);let d=t.get(c);if(!d)throw new Error(`scene node ${c} is missing`);n.add(c);let u=d.content,h;if(u.kind==="row"||u.kind==="column"||u.kind==="box"){let g=u.children.map(S=>this.#o(S,t,i,n,o,s,a)),f=(d.layout.gap??o.uiSkin.layout.containerGapPx)*a;u.kind==="row"?h={width:g.reduce((S,_)=>S+_.width,0)+Math.max(0,g.length-1)*f,height:Math.max(0,...g.map(S=>S.height))}:u.kind==="column"?h={width:Math.max(0,...g.map(S=>S.width)),height:g.reduce((S,_)=>S+_.height,0)+Math.max(0,g.length-1)*f}:h={width:Math.max(0,...g.map(S=>S.width)),height:Math.max(0,...g.map(S=>S.height))}}else h=this.#r(u,o,s,a);n.delete(c);let p=d.layout.aspectRatio;if(p&&p.width>0&&p.height>0){let g=p.width/p.height;h=h.width/Math.max(h.height,1)>g?{width:h.width,height:h.width/g}:{width:h.height*g,height:h.height}}let x=Object.freeze(h);return i.set(c,x),x}#r(e,t,i,n){let o=t.typography,s=t.uiSkin;if(e.kind==="viewport"){let P=t.uiReferenceSize,w=Math.min(P.widthPx,P.heightPx*(4/3));return{width:w*n,height:w*(3/4)*n}}if(e.kind==="frame")return{width:1,height:1};if(e.kind==="audio_controls")return{width:240*n,height:40*n};let a=e.kind==="button"||e.kind==="toggle",c=e.kind==="text"?String(e.role):(e.kind==="error","body"),l=o[c],d=l.fontSizePx*n,u=e.kind==="text"?String(e.value):e.kind==="button"||e.kind==="toggle"?String(e.label):String(e.message??""),h=s.control.layout,p=a?h.paddingHorizontalPx*n:0,x=a?h.paddingVerticalPx*n:0,g=a?h.marginPx*n:0,m=a?h.widthPx:null,f=l.maxWidthPx,S=m!==null?Math.max(1,m*n-p*2):f===null?null:f*n,_=d*l.lineHeight,R=this.#s(i,u,d,_,S);return{width:(m===null?R.width+p*2:m*n)+g*2,height:R.height+x*2+g*2,text:Object.freeze({lines:R.lines,lineHeight:R.lineHeight,width:R.width})}}#s(e,t,i,n,o){let s=new Map,a=h=>{let p=s.get(h);if(p)return p;let x=this.#e.measureShapedText(e,h,i);if(x.status==="pending")throw new In;if(x.status==="failed")throw new Error(x.message);try{let g=Object.freeze({width:x.lease.width,height:x.lease.height});return s.set(h,g),g}finally{x.lease.release()}},c=h=>a(h).width,l=[];for(let h of t.split(`
`)){if(o===null){l.push(h);continue}let p=h.split(/(\s+)/u).filter(g=>g.length>0),x="";for(let g of p){let m=x+g;if(c(m)<=o){x=m;continue}if(x.trimEnd().length>0&&l.push(x.trimEnd()),x=g.trimStart(),x.length===0||c(x)<=o)continue;let f="";for(let S of Array.from(x)){let _=f+S;f.length>0&&c(_)>o?(l.push(f),f=S):f=_}x=f}l.push(x)}l.length===0&&l.push("");let d=l.map(a),u=Math.max(n,1,...d.map(h=>h.height));return Object.freeze({width:Math.max(1,...d.map(h=>h.width)),height:u*l.length,lineHeight:u,lines:Object.freeze(l)})}#a(e){let t=Pe(e.nodeId),i=e.nodes.get(t),n=i.content,o=lr(e.rect,e.clip);if(!o&&!e.retainClippedResources)return;let s=o??e.clip;if(n.kind==="row"||n.kind==="column"||n.kind==="box"){let a=n.children,c=i.layout,l=(c.gap??e.theme.uiSkin.layout.containerGapPx)*e.scale,d=n.kind==="column",u=d?e.rect.height:e.rect.width,h=a.reduce((v,I)=>{let F=e.measurements.get(Pe(I));return v+(d?F.height:F.width)},0)+Math.max(0,a.length-1)*l,p=u-h,x=a.reduce((v,I)=>{let E=e.nodes.get(Pe(I)).layout.space;return v+(E.kind==="fill"?E.weight:0)},0),g=Math.max(0,p),m=d?e.rect.y:e.rect.x;c.distribute==="center"&&(m+=g/2),c.distribute==="end"&&(m+=g);let f=c.distribute==="between"&&a.length>1?g/(a.length-1):0,S=[];for(let v of a){let I=e.measurements.get(Pe(v)),F=e.nodes.get(Pe(v)).layout,E=F.space,N=E.kind==="fill"&&x>0?p*E.weight/x:0,W=d?br(I.width,e.rect.width,F,c):I.width+N,X=d?I.height+N:br(I.height,e.rect.height,F,c),te=d?Rr(e.rect.x,e.rect.width,W,F,c):m,V=d?m:Rr(e.rect.y,e.rect.height,X,F,c);S.push({id:v,rect:{x:te,y:V,width:Math.max(1,W),height:Math.max(1,X)}}),m+=(d?X:W)+l+f}if(n.kind==="box"){S.length=0;for(let v of a){let I=e.measurements.get(Pe(v)),F=e.nodes.get(Pe(v)).layout,E=br(I.width,e.rect.width,F,c),N=br(I.height,e.rect.height,F,c);S.push({id:v,rect:{x:Rr(e.rect.x,e.rect.width,E,F,c),y:Rr(e.rect.y,e.rect.height,N,F,c),width:E,height:N}})}}let _=Math.max(e.rect.x+e.rect.width,...S.map(v=>v.rect.x+v.rect.width)),R=Math.max(e.rect.y+e.rect.height,...S.map(v=>v.rect.y+v.rect.height)),P=Math.max(0,_-e.rect.x-e.rect.width),w=Math.max(0,R-e.rect.y-e.rect.height),T=`node:${t}`,C=c.scroll===!0,y=C?this.#i.scrollOffset(T,P,w):{x:0,y:0};C&&(P>0||w>0)&&e.scrollableRegions.push(Object.freeze({key:T,rect:s,maximumX:P,maximumY:w,lineHeight:e.theme.typography.body.fontSizePx*e.theme.typography.body.lineHeight*e.scale}));for(let v of S)this.#a({...e,nodeId:v.id,rect:{...v.rect,x:v.rect.x-y.x,y:v.rect.y-y.y},clip:s});return}this.#l(e,i,n,s)}#c(e){let t=Pe(e.nodeId),i=e.nodes.get(t),n=i.content;if(n.kind!=="row"&&n.kind!=="column"&&n.kind!=="box"){if(n.kind!=="button"&&n.kind!=="toggle")return null;let C=n.control.activation;return n.selected===!0||C.kind!=="idle"?e.rect:null}let o=n.children,s=i.layout,a=(s.gap??e.theme.uiSkin.layout.containerGapPx)*e.scale,c=n.kind==="column",l=c?e.rect.height:e.rect.width,d=o.reduce((C,y)=>{let v=e.measurements.get(Pe(y));return C+(c?v.height:v.width)},0)+Math.max(0,o.length-1)*a,u=l-d,h=o.reduce((C,y)=>{let I=e.nodes.get(Pe(y)).layout.space;return C+(I.kind==="fill"?I.weight:0)},0),p=Math.max(0,u),x=c?e.rect.y:e.rect.x;s.distribute==="center"&&(x+=p/2),s.distribute==="end"&&(x+=p);let g=s.distribute==="between"&&o.length>1?p/(o.length-1):0,m=[];for(let C of o){let y=e.measurements.get(Pe(C)),v=e.nodes.get(Pe(C)).layout,I=v.space,F=I.kind==="fill"&&h>0?u*I.weight/h:0,E=c?br(y.width,e.rect.width,v,s):y.width+F,N=c?y.height+F:br(y.height,e.rect.height,v,s);m.push({id:C,rect:{x:c?Rr(e.rect.x,e.rect.width,E,v,s):x,y:c?x:Rr(e.rect.y,e.rect.height,N,v,s),width:Math.max(1,E),height:Math.max(1,N)}}),x+=(c?N:E)+a+g}if(n.kind==="box"){m.length=0;for(let C of o){let y=e.measurements.get(Pe(C)),v=e.nodes.get(Pe(C)).layout,I=br(y.width,e.rect.width,v,s),F=br(y.height,e.rect.height,v,s);m.push({id:C,rect:{x:Rr(e.rect.x,e.rect.width,I,v,s),y:Rr(e.rect.y,e.rect.height,F,v,s),width:I,height:F}})}}let f=null;for(let C of m)if(f=this.#c({...e,nodeId:C.id,rect:C.rect}),f)break;if(!f)return null;if(s.scroll!==!0)return lr(f,e.rect);let S=Math.max(e.rect.x+e.rect.width,...m.map(C=>C.rect.x+C.rect.width)),_=Math.max(e.rect.y+e.rect.height,...m.map(C=>C.rect.y+C.rect.height)),R=Math.max(0,S-e.rect.x-e.rect.width),P=Math.max(0,_-e.rect.y-e.rect.height),w=`node:${t}`;this.#i.reveal(w,R,P,e.rect,f);let T=this.#i.scrollOffset(w,R,P);return Object.freeze({...f,x:f.x-T.x,y:f.y-T.y})}#l(e,t,i,n){let o=e.theme.uiSkin,s=e.theme.typography;if(i.kind==="viewport"){let C=e.viewportTargets.get(Pe(i.viewport));if(!C)throw new Error(`viewport ${Pe(i.viewport)} is missing`);let y=this.#f(e.generationId,C,e.rect,n,e.theme);if(y.status!=="ready")throw new Error(y.message);e.viewports.push(y.viewport),y.cameraRegion&&e.cameraRegions.push(y.cameraRegion);return}if(i.kind==="frame"){e.underlay.push(Xs(`frame:${Pe(t.id)}`,BigInt(e.underlay.length),e.rect,o.panel,e.clip,e.scale));return}if(i.kind==="audio_controls"){e.underlay.push(Xs(`audio:${Pe(t.id)}`,BigInt(e.underlay.length),e.rect,o.panel,e.clip,e.scale));return}let a=i.kind==="button"||i.kind==="toggle",c=e.rect,l=null,d=null;if(a){let C=o.control,y=C.layout,v=i.control;d=v.activation.kind!=="idle"?e.theme.interactionInk.selected:i.selected===!0?e.theme.interactionInk.focus:null;let E=y.marginPx*e.scale;c=rm(e.rect,E),d?.kind==="lift"&&(c=Object.freeze({...c,y:c.y-d.liftPx*e.scale}));let N=v.enabled===!0,W=e.theme.interactionInk.disabledOpacity,X=d?d.fill:C.fill,te=d?d.border:C.border;l=N?d?d.foreground:C.text:o.mutedText,e.underlay.push(Xs(`control:${Pe(t.id)}`,BigInt(e.underlay.length),c,Object.freeze({fill:N?X:Jp(X,W),border:N?te:Jp(te,W),borderWidthPx:y.borderWidthPx,cornerRadiusPx:y.cornerRadiusPx,shadow:Jv}),e.clip,e.scale,d?.sketch));let V=lr(c,e.clip);V&&v.enabled===!0&&e.hitRegions.push(Object.freeze({rect:V,logical:v.id}))}let u=i.kind==="text"?String(i.role):"body",h=s[u],p=h.fontSizePx*e.scale,x=i.kind==="text"?String(i.value):i.kind==="button"||i.kind==="toggle"?String(i.label):String(i.message??""),g=i.kind==="error"?o.accent:a?l:o.text,m=a?c:e.rect,f=a||i.kind==="text"&&(i.textAlign??h.alignment)==="center"?m.x+m.width/2:i.kind==="text"&&(i.textAlign??h.alignment)==="end"?m.x+m.width:m.x,S=a?"center":i.kind==="text"&&(i.textAlign??h.alignment)==="center"?"top":i.kind==="text"&&(i.textAlign??h.alignment)==="end"?"top_right":"top_left",_=e.measurements.get(Pe(t.id)),R=_?.text?.lines??[x],P=_?.text?.lineHeight??p*h.lineHeight,w=P*R.length,T=a?m.y+(m.height-w)/2+P/2:m.y;for(let C=0;C<R.length;C+=1)e.overlay.push(Cc(`text:${Pe(t.id)}:${C}`,BigInt(e.overlay.length),e.fontKey,R[C],p,P,S,g,f,T+P*C,e.clip));a&&this.#d(e,t,i,c,l,p,_)}#d(e,t,i,n,o,s,a){let c=i.control,d=c.activation.kind!=="idle";if(c.enabled!==!0||i.selected!==!0&&!d)return;let u=e.theme.uiSkin.control.layout,h=u.selectionMarker;if(!h)return;let p=String(d?h.selectedGlyph:h.idleGlyph),x=u.paddingHorizontalPx*e.scale,g=h.columnGapPx*e.scale,m=a?.text?.width??this.#u(e.fontKey,String(i.label),s),f=Math.max(0,(n.width-x*2-m-g*2)/2),S=p;if(d&&h.selectedFill===!0){let T=this.#u(e.fontKey,p,s),C=T>Number.EPSILON?Math.max(1,Math.floor(f/T)):1;S=p.repeat(C)}let _=n.y+n.height/2,R=a?.text?.lineHeight??s,P=n.x+(n.width-m)/2,w=P+m;e.overlay.push(Cc(`marker-left:${Pe(t.id)}`,BigInt(e.overlay.length),e.fontKey,S,s,R,"right",o,P-g,_,e.clip)),e.overlay.push(Cc(`marker-right:${Pe(t.id)}`,BigInt(e.overlay.length),e.fontKey,S,s,R,"left",o,w+g,_,e.clip))}#u(e,t,i){let n=this.#e.measureShapedText(e,t,i);if(n.status==="pending")throw new In;if(n.status==="failed")throw new Error(n.message);try{return n.lease.width}finally{n.lease.release()}}#f(e,t,i,n,o){let s=t.projection,a=[];for(let p of t.batches){let x=this.projectResolvedBatch(e,p);if(x.status==="failed")return x;a.push(x.batch)}let c=t.decorations.map(p=>Ev(p,s,i));if(s.kind==="two_d"){let p=s.origin,x=s.size;return{status:"ready",viewport:Object.freeze({key:Pe(t.id),framebuffer:i,clipPixels:n,camera:Object.freeze({kind:"orthographic",projectionMatrix:tm(p,x,i),viewMatrix:Ys}),treatment:Object.freeze({render:qs(o.renderTreatment),threeD:null}),batches:Object.freeze(a),decorations:Object.freeze(c)}),cameraRegion:null}}let l=s.camera,d=Pe(t.id),u=this.#i.camera(d),h=Dv(l,i,u,Lv(a,c));return{status:"ready",viewport:Object.freeze({key:d,framebuffer:i,clipPixels:n,camera:h,treatment:Object.freeze({render:qs(s.renderTreatment),threeD:Object.freeze({lighting:Av(s.lighting),shade:s.shade,shadow:s.shadow,pixelate:s.pixelate})}),batches:Object.freeze(a),decorations:Object.freeze(c)}),cameraRegion:Object.freeze({key:d,rect:n,interactiveLook:l.interactiveLook,interactiveZoom:l.interactiveZoom})}}projectResolvedBatch(e,t){let i=t.content,n;if(i.kind==="pixels"){let o=t.pixelGeometry;if(!o)return{status:"failed",message:"pixel batch is missing exact pixelGeometry"};let s=i.positions,a=i.palette;n={kind:"pixels",width:i.width,height:i.height,geometry:{x:o.x,y:o.y,width:o.width,height:o.height,clip:o.clip},pixels:i.paletteIndices.map((c,l)=>({position:[s[l*2],s[l*2+1]],color:{red:a[c*4],green:a[c*4+1],blue:a[c*4+2],alpha:a[c*4+3]}}))}}else if(i.kind==="voxels"){let o=i.width,s=i.depth,a=i.height,c=1/Math.max(o,s,a);n={kind:"voxels",width:o,depth:s,height:a,geometry:{origin:[-o*c/2,-s*c/2,-a*c/2],cellSize:[c,c,c]},voxels:i.voxels.map(l=>({position:l.position,color:l.color}))}}else if(i.kind==="raster_image"){let o=xo(e,i.asset,i.revision,i.frame);if(!this.#e.hasImageFrameResource(o))return{status:"failed",message:`unknown installed image frame ${o}`};n={kind:"raster_image",resourceKey:o,destination:i.destination,uv:i.uv,sampling:i.sampling}}else{let o=i.font,s=this.#t(e,o,"$.batch.content.font");if(s.status!=="ready")return{status:"failed",message:s.message};n={kind:"text",resourceKey:s.key,value:i.value,fontSize:i.fontSize,lineHeight:i.fontSize*1.2,align:i.align,overflow:i.overflow,units:"cell",color:i.color}}return{status:"ready",batch:Object.freeze({key:Pe(t.key),renderOrder:BigInt(t.key.renderOrder),drawIndex:BigInt(t.drawIndex),cell:t.key.cell,coordinateSpace:"cell",transform:t.transform,opacity:t.opacity,clipPixels:null,content:n})}}};function Xs(r,e,t,i,n,o,s=null){let a=i.shadow;return Object.freeze({key:r,renderOrder:e,drawIndex:e,cell:[0,0,0],coordinateSpace:"surface_pixels",transform:Ys,opacity:1,clipPixels:n,content:Object.freeze({kind:"surface_shape",shape:Object.freeze({rect:Object.freeze({...t}),fill:i.fill,border:i.border,borderWidthPx:i.borderWidthPx*o,cornerRadiusPx:i.cornerRadiusPx*o,shadow:Object.freeze({color:a.color,offsetXPx:a.offsetXPx*o,offsetYPx:a.offsetYPx*o,blurPx:a.blurPx*o}),sketch:s===null?null:Object.freeze({underlineStrokePx:s.underlineStrokePx*o,outlineStrokePx:s.outlineStrokePx*o,roughness:s.roughness,seed:Gv(r)})})})})}function Cc(r,e,t,i,n,o,s,a,c,l,d){let u=[...Ys];return u[12]=c,u[13]=l,Object.freeze({key:r,renderOrder:e,drawIndex:e,cell:[0,0,0],coordinateSpace:"surface_pixels",transform:u,opacity:1,clipPixels:d,content:Object.freeze({kind:"text",resourceKey:t,value:i,fontSize:n,lineHeight:o,align:s,overflow:"clip",units:"surface_pixels",color:a})})}function Kp(r,e,t,i){return Object.freeze({key:r,framebuffer:Object.freeze({x:0,y:0,width:e.width,height:e.height}),clipPixels:Object.freeze({x:0,y:0,width:e.width,height:e.height}),camera:Object.freeze({kind:"orthographic",projectionMatrix:Nv(e),viewMatrix:Ys}),treatment:Object.freeze({render:i,threeD:null}),batches:Object.freeze([...t]),decorations:Object.freeze([])})}function Ev(r,e,t){let i=e.kind==="two_d"?(e.size[1]??1)/Math.max(t.height,1):1/Math.max(t.height,1);return Uc(r,i)}function Uc(r,e){let t=r.style,i=t?.color??r.color,n=t?Iv(t.width,e):0,o=t?Cv(t.treatment):Object.freeze({kind:"uniform"});if(r.kind==="lines_2d")return Object.freeze({kind:"lines_2d",segments:r.segments,widthWorld:n,color:i,treatment:o});if(r.kind==="lines_3d"){let a=r.segments.map(c=>Object.freeze({start:c.start,end:c.end}));return Object.freeze({kind:"lines_3d",segments:Object.freeze(a),widthWorld:n,color:i,depth:r.depth,treatment:o})}let s=r.triangles.map(a=>Object.freeze({points:Object.freeze(a.points)}));return Object.freeze({kind:"triangles_3d",triangles:Object.freeze(s),color:i,depth:r.depth})}function Av(r){let e=Dn(r.yawDegrees),t=Dn(r.pitchDegrees),i=Object.freeze([Math.sin(e)*Math.cos(t),-Math.cos(e)*Math.cos(t),Math.sin(t)]);return Object.freeze({intensity:r.intensity,ambient:r.ambient,sourceDirection:i,color:r.color})}function Cv(r){return r.kind==="uniform"?Object.freeze({kind:"uniform"}):Object.freeze({kind:"graphite",roughness:r.roughness,pressureVariation:r.pressureVariation,overdraw:r.overdraw,seed:r.seed})}function qs(r){return Object.freeze({sampling:r.sampling,pixelSnapping:r.pixelSnapping,shadow:r.shadow,outline:r.outline,outlineWidthPx:r.outlineWidthPx,postEffect:r.postEffect,postEffectStrength:r.postEffectStrength})}function Pv(r){if(r.kind==="none")return null;let e=r.grain,t=r.ruling;return Object.freeze({kind:"paper",grain:Object.freeze({scalePx:e.scalePx,strength:e.strength,erasureCount:e.erasureCount,erasureScalePx:e.erasureScalePx}),ruling:Object.freeze({kind:t.kind,spacingPx:t.spacingPx,lineWidthPx:t.lineWidthPx,color:t.color})})}function Iv(r,e){return r.kind==="physical_pixels"?r.pixels*e:Math.max(r.cellFraction,r.minPhysicalPixels*e)}function Dv(r,e,t,i){let n=r.framing,o=n?_o(n.center):nm(i),s=n?Wv(n.size):Dc(i),a=Dc(i),c=Math.max(Xv(a)/2,.75),l=Qp(Qp(Ic([0,1,0],Dn(r.yawDegrees+t.yaw)),Ic([1,0,0],-Dn(r.pitchDegrees+t.pitch))),Ic([0,0,1],Dn(r.rollDegrees))),d=r.zoom*t.zoom,u=e.width/Math.max(e.height,1),h=om(l),p=Ov(o,s).map(S=>vo(Lc(S,o),h)),x=Math.max(...Fv(i).map(S=>vo(Lc(S,o),h)[2]+.1),Number.MIN_VALUE),g=Math.max(1e3,c*100);if(r.projection==="perspective"){let S=Math.tan(Math.PI/8),_=S*u,R=Math.max(...p.map(w=>Math.max(w[2]+Math.abs(w[0])/_,w[2]+Math.abs(w[1])/S,w[2]+.1)))/d,P=yo(o,vo([0,0,Math.max(R,.1)],l));return Object.freeze({kind:"perspective",projectionMatrix:Kv(45,u,.1,g),viewMatrix:em(P,l)})}let m=Math.max(...p.map(S=>Math.max(Math.abs(S[1]),Math.abs(S[0])/u)))/d,f=yo(o,vo([0,0,x],l));return Object.freeze({kind:"orthographic",projectionMatrix:Zv(-m*u,m*u,m,-m,.1,g),viewMatrix:em(f,l)})}function Lv(r,e){let t=Hv();for(let s of r){if(s.content.kind!=="voxels")continue;let a=sm(qv(s.cell),s.transform),{origin:c,cellSize:l}=s.content.geometry;for(let d of s.content.voxels){if(d.color.alpha*s.opacity<=0)continue;let u=[c[0]+d.position[0]*l[0],c[1]+d.position[1]*l[1],c[2]+d.position[2]*l[2]],h=yo(u,l);Uv(t,u,h,a)}}for(let s of e)if(s.kind==="lines_3d")for(let a of s.segments)$s(t,_o(a.start)),$s(t,_o(a.end));else if(s.kind==="triangles_3d")for(let a of s.triangles)for(let c of a.points)$s(t,_o(c));if(jv(t))return{min:[-.5,-.5,-.5],max:[.5,.5,.5]};let i=nm(t),n=Dc(t),o=[Math.max(n[0]/2,.01),Math.max(n[1]/2,.01),Math.max(n[2]/2,.01)];return{min:Lc(i,o),max:yo(i,o)}}function Uv(r,e,t,i){for(let n of[e[0],t[0]])for(let o of[e[1],t[1]])for(let s of[e[2],t[2]])$s(r,_o($v(i,[n,o,s])))}function Fv(r){let e=[];for(let t of[r.min[0],r.max[0]])for(let i of[r.min[1],r.max[1]])for(let n of[r.min[2],r.max[2]])e.push([t,i,n]);return e}function Ov(r,e){let t=[e[0]/2,e[1]/2,e[2]/2],i=[];for(let n of[-1,1])for(let o of[-1,1])for(let s of[-1,1])i.push(yo(r,[t[0]*n,t[1]*o,t[2]*s]));return i}function tm(r,e,t){let i=Math.max(e[0],1),n=Math.max(e[1],1),o=Math.max(t.width,1)/Math.max(t.height,1),s=Math.max(i,n*o),a=Math.max(n,i/o),c=r[0]-(s-i)/2,l=r[1]-(a-n)/2;return[2/s,0,0,0,0,-2/a,0,0,0,0,-1,0,-1-2*c/s,1+2*l/a,0,1]}function Nv(r){return tm([0,0],[r.width,r.height],r)}function Bv(r,e){let t=r.uiReferenceSize;return Math.max(1e-4,Math.min(e.width/Math.max(t.widthPx,1),e.height/Math.max(t.heightPx,1)))}function kv(r,e,t){let i=r.uiReferenceSize,n=i.widthPx*t,o=i.heightPx*t;return Object.freeze({x:(e.width-n)/2,y:(e.height-o)/2,width:n,height:o})}function zv(r,e,t,i){if(r.modal!==!0&&r.placement!=="overlay")return e;let n=t.layout,o=Math.min(e.width,n.modalMaxWidthPx*i);return rm(Object.freeze({x:e.x+(e.width-o)/2,y:e.y,width:o,height:e.height}),n.modalPaddingPx*i)}function Vv(r,e){return Zp(r)-Zp(e)}function Zp(r){return r.modal===!0?3:r.placement==="root"?0:r.placement==="content"?1:2}function br(r,e,t,i){return im(t,i)==="stretch"?e:Math.min(r,e)}function Rr(r,e,t,i,n){let o=im(i,n);return o==="center"?r+(e-t)/2:o==="end"?r+e-t:r}function im(r,e){return r.alignSelf??e.align}function Jp(r,e){return Object.freeze({...r,alpha:r.alpha*e})}function rm(r,e){let t=Math.max(1,r.width-e*2),i=Math.max(1,r.height-e*2);return Object.freeze({x:r.x+(r.width-t)/2,y:r.y+(r.height-i)/2,width:t,height:i})}function Gv(r){let e=2166136261;for(let t=0;t<r.length;t+=1)e^=r.charCodeAt(t),e=Math.imul(e,16777619);return e>>>0}function Pc(r,e,t){return e>=r.x&&t>=r.y&&e<r.x+r.width&&t<r.y+r.height}function Dn(r){return r*Math.PI/180}function _o(r){return[r[0],r[2],-r[1]]}function Wv(r){return[r[0],r[2],r[1]]}function Hv(){return{min:[Number.POSITIVE_INFINITY,Number.POSITIVE_INFINITY,Number.POSITIVE_INFINITY],max:[Number.NEGATIVE_INFINITY,Number.NEGATIVE_INFINITY,Number.NEGATIVE_INFINITY]}}function jv(r){return r.max[0]<r.min[0]||r.max[1]<r.min[1]||r.max[2]<r.min[2]}function $s(r,e){r.min=[Math.min(r.min[0],e[0]),Math.min(r.min[1],e[1]),Math.min(r.min[2],e[2])],r.max=[Math.max(r.max[0],e[0]),Math.max(r.max[1],e[1]),Math.max(r.max[2],e[2])]}function nm(r){return[(r.min[0]+r.max[0])/2,(r.min[1]+r.max[1])/2,(r.min[2]+r.max[2])/2]}function Dc(r){return[r.max[0]-r.min[0],r.max[1]-r.min[1],r.max[2]-r.min[2]]}function yo(r,e){return[r[0]+e[0],r[1]+e[1],r[2]+e[2]]}function Lc(r,e){return[r[0]-e[0],r[1]-e[1],r[2]-e[2]]}function Xv(r){return Math.hypot(r[0],r[1],r[2])}function Ic(r,e){let t=e/2,i=Math.sin(t);return[r[0]*i,r[1]*i,r[2]*i,Math.cos(t)]}function Qp(r,e){let[t,i,n,o]=r,[s,a,c,l]=e;return[t*l+o*s+i*c-n*a,i*l+o*a+n*s-t*c,n*l+o*c+t*a-i*s,o*l-t*s-i*a-n*c]}function om(r){let e=r[0]**2+r[1]**2+r[2]**2+r[3]**2;return[-r[0]/e,-r[1]/e,-r[2]/e,r[3]/e]}function vo(r,e){let[t,i,n]=r,[o,s,a,c]=e,l=c*t+s*n-a*i,d=c*i+a*t-o*n,u=c*n+o*i-s*t,h=-o*t-s*i-a*n;return[l*c+h*-o+d*-a-u*-s,d*c+h*-s+u*-o-l*-a,u*c+h*-a+l*-s-d*-o]}function qv(r){return[1,0,0,0,0,1,0,0,0,0,1,0,r[0],r[1],r[2],1]}function sm(r,e){let t=new Array(16).fill(0);for(let i=0;i<4;i+=1)for(let n=0;n<4;n+=1){let o=0;for(let s=0;s<4;s+=1)o+=r[s*4+n]*e[i*4+s];t[i*4+n]=o}return Object.freeze(t)}function $v(r,e){let t=e[0],i=e[1],n=e[2],o=r[3]*t+r[7]*i+r[11]*n+r[15],s=o===0?1:1/o;return[(r[0]*t+r[4]*i+r[8]*n+r[12])*s,(r[1]*t+r[5]*i+r[9]*n+r[13])*s,(r[2]*t+r[6]*i+r[10]*n+r[14])*s]}function Yv(r,e){let[t,i,n,o]=r,s=t+t,a=i+i,c=n+n,l=t*s,d=t*a,u=t*c,h=i*a,p=i*c,x=n*c,g=o*s,m=o*a,f=o*c;return[1-(h+x),d+f,u-m,0,d-f,1-(l+x),p+g,0,u+m,p-g,1-(l+h),0,e[0],e[1],e[2],1]}function em(r,e){let t=om(e),i=Yv(t,vo([-r[0],-r[1],-r[2]],t));return sm(i,Qv)}function Kv(r,e,t,i){let o=2*(t*Math.tan(Dn(r)/2)),s=e*o,a=-s/2,c=2*t/s,l=2*t/o,d=-(i+t)/(i-t),u=-2*i*t/(i-t);return[c,0,0,0,0,l,0,0,0,0,d,-1,-2*a/s-1,0,u,0]}function Zv(r,e,t,i,n,o){let s=1/(e-r),a=1/(t-i),c=1/(o-n);return[2*s,0,0,0,0,2*a,0,0,0,0,-2*c,0,-(e+r)*s,-(t+i)*a,-(o+n)*c,1]}function Pn(r,e,t){return{status:"failed",code:r,path:e,message:t}}var In=class extends Error{},Jv=Object.freeze({color:Object.freeze({red:0,green:0,blue:0,alpha:0}),offsetXPx:0,offsetYPx:0,blurPx:0}),Ys=Object.freeze([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]),Qv=Object.freeze([1,0,0,0,0,0,-1,0,0,1,0,0,0,0,0,1]);var Ks=class{decode(){return Qe("invalid_schema","$","browser facade must use its method-specific closed decoder")}revisionKey(e){return mi(e)}sameApplyPayload(e,t,i,n){return cr(e,i)&&cr(t,n)}compactApplyPayload(e,t,i,n){return ul({baseRevision:e,targetRevision:t,target:i,diff:n},"apply-replay:v1")}sameCompactedApplyPayload(e,t,i,n,o){return e===Lo({baseRevision:t,targetRevision:i,target:n,diff:o},"apply-replay:v1")}install(e,t){return Zt(t,e.revision)?am(e,null):Qe("target_diff_disagreement","$.target.revision","target revision disagrees with Install")}apply(e,t,i,n,o){if(!Zt(e.revision,n))return Qe("wrong_base_revision","$.baseRevision","named base does not match the retained root");if(!Zt(t.revision,o))return Qe("target_diff_disagreement","$.target.revision","target revision disagrees with Apply");if(!Zt(i.baseRevision,n)||!Zt(i.targetRevision,o))return Qe("target_diff_disagreement","$.diff","diff revisions disagree with Apply envelope");let s=i.kind==="stable"?e_(e,t,i):t_(e,t,i);return s.ok?am(t,e):s}validateSample(e,t){let i=i_(e,t.wire);return i.ok?{ok:!0,value:Object.freeze({wire:t.wire,composed:i.value})}:i}};function am(r,e){let t=Ti(r.resources,"$.target.resources",So);if(!t.ok)return t;let i=Ti(r.viewports,"$.target.viewports",bo);if(!i.ok)return i;let n=cm(r.surface,"$.target.surface");if(!n.ok)return n;for(let[l,d]of i.value){let u=lm(d,`$.target.viewports[${l}]`);if(!u.ok)return u}let o=r_(t.value,e?.resources??null),s=n_(i.value,e?.viewports??null),a=Js(r.configuration,e?.configuration??null),c=o_(r.surface,e?.surface??null);return{ok:!0,value:Object.freeze({revision:r.revision,target:r,configuration:a,resources:o,surface:c,viewports:s})}}function e_(r,e,t){let i=dm(r.resources,t.resources);if(!i.ok)return i;let n=Ti(e.resources,"$.target.resources",So);if(!n.ok)return n;if(!Zs(i.value,n.value))return Qe("target_diff_disagreement","$.diff.resources","resource delta does not equal complete target resources");let o=t.targetConfiguration===null?r.configuration:t.targetConfiguration,s=t.targetSurface===null?r.surface:t.targetSurface;if(!Zt(o,e.configuration))return Qe("target_diff_disagreement","$.diff.targetConfiguration","configuration diff does not equal target");if(!Zt(s,e.surface))return Qe("target_diff_disagreement","$.diff.targetSurface","surface diff does not equal target");let a=Ti(e.viewports,"$.target.viewports",bo);if(!a.ok)return a;let c=new Map(r.viewports),l=new Set;for(let d of t.changedViewports){let u=Pe(d);if(l.has(u))return wi("$.diff.changedViewports",u);l.add(u);let h=r.viewports.get(u),p=a.value.get(u);if(!h||!p)return Qe("unknown_reference","$.diff.changedViewports",`stable diff names viewport ${u} that is absent from its base or complete target`);if(Zt(h,p))return Qe("target_diff_disagreement","$.diff.changedViewports",`stable diff falsely marks unchanged viewport ${u} as changed`);c.set(u,p)}if(!Zs(r.viewports,a.value))return Qe("incomplete_diff","$.diff.changedViewports","stable diff cannot change the viewport identity set");for(let[d,u]of a.value)if(!Zt(c.get(d),u))return Qe("incomplete_diff","$.diff.changedViewports",`stable diff omits changed viewport ${d}`);return{ok:!0,value:!0}}function t_(r,e,t){let i=dm(r.resources,t.resources);if(!i.ok)return i;let n=Ti(e.resources,"$.target.resources",So);if(!n.ok)return n;if(!Zs(i.value,n.value))return Qe("target_diff_disagreement","$.diff.resources","resource delta does not equal target resources");if(!Zt(t.targetConfiguration,e.configuration)||!Zt(t.targetSurface,e.surface))return Qe("target_diff_disagreement","$.diff","reconfigure values do not equal complete target");let o=Ti(e.viewports,"$.target.viewports",bo);if(!o.ok)return o;let s=new Map;for(let a of t.retainedViewports){let c=Pe(a);if(s.has(c))return wi("$.diff.retainedViewports",c);let l=r.viewports.get(c);if(!l)return Qe("unknown_reference","$.diff.retainedViewports",`unknown retained viewport ${c}`);s.set(c,l)}for(let a of t.replacementViewports){let c=bo(a);if(s.has(c))return wi("$.diff.replacementViewports",c);s.set(c,a)}if(!Zs(s,o.value))return Qe("incomplete_diff","$.diff","reconfigure viewport set does not equal complete target");for(let[a,c]of s)if(!Zt(c,o.value.get(a)))return Qe("target_diff_disagreement","$.diff",`viewport ${a} differs from complete target`);return{ok:!0,value:!0}}function i_(r,e){let t=e.surfaceOverride??r.surface,i=Ti(t.nodes,"$.surfaceOverride.nodes",er);if(!i.ok)return i;let n=new Set;for(let h of e.nodePatches){let p=er(h.nodeId);if(n.has(p))return wi("$.nodePatches",p);if(n.add(p),h.replacement===null){if(!i.value.delete(p))return Qe("unknown_reference","$.nodePatches",`cannot remove unknown node ${p}`)}else{if(er(h.replacement.id)!==p)return Qe("target_diff_disagreement","$.nodePatches","replacement node identity differs from patch identity");i.value.set(p,h.replacement)}}let o=n.size===0?t:Object.freeze({...t,nodes:Object.freeze([...i.value.values()])}),s=cm(o,"$.sample.surface");if(!s.ok)return s;let a=new Map(r.viewports),c=new Set;for(let h of e.removeViewports){let p=Pe(h);if(c.has(p))return wi("$.removeViewports",p);if(c.add(p),!a.delete(p))return Qe("unknown_reference","$.removeViewports",`cannot remove unknown viewport ${p}`)}let l=new Set;for(let h of e.upsertViewports){let p=bo(h);if(l.has(p)||c.has(p))return wi("$.upsertViewports",p);l.add(p),a.set(p,h)}let d=new Set;for(let h of e.viewportPatches){let p=Pe(h.viewport);if(d.has(p)||c.has(p)||l.has(p))return wi("$.viewportPatches",p);d.add(p);let x=a.get(p);if(!x)return Qe("unknown_reference","$.viewportPatches",`unknown viewport ${p}`);let g=Ti(x.batches,"$.viewportPatches.batches",Ro);if(!g.ok)return g;let m=new Set;for(let R of h.removeBatchKeys){let P=Pe(R);if(m.has(P)||!g.value.delete(P))return Qe("duplicate_identity","$.viewportPatches.removeBatchKeys",`duplicate or unknown batch ${P}`);m.add(P)}let f=new Set;for(let R of h.upsertBatches){let P=Ro(R);if(f.has(P)||m.has(P))return wi("$.viewportPatches.upsertBatches",P);f.add(P),g.value.set(P,R)}let S=Object.freeze({...x,projection:h.projection??x.projection,batches:Object.freeze([...g.value.values()]),decorations:h.targetDecorations??x.decorations}),_=lm(S,"$.viewportPatches");if(!_.ok)return _;a.set(p,S)}let u=BigInt(e.interactionRevision);return{ok:!0,value:Object.freeze({wire:e,configuration:e.configurationOverride??r.configuration,resourceBindings:e.resourceBindings,surface:o,viewports:Object.freeze([...a.values()]),extent:e.extent,interactionRevision:u})}}function cm(r,e){let t=Ti(r.components,`${e}.components`,a=>String(a.id));if(!t.ok)return t;let i=Ti(r.nodes,`${e}.nodes`,er);if(!i.ok)return i;for(let a of t.value.values()){let c=er(a.rootNode);if(!i.value.has(c))return Qe("unknown_reference",`${e}.components`,`component root ${c} is missing`)}for(let a of i.value.values()){let l=a.content.children;if(l)for(let d of l){let u=er(d);if(!i.value.has(u))return Qe("unknown_reference",`${e}.nodes`,`child node ${u} is missing`)}}let n=new Set,o=new Set,s=a=>{if(n.has(a))return{ok:!0,value:!0};if(o.has(a))return Qe("invalid_schema",`${e}.nodes`,`scene node cycle reaches ${a}`);o.add(a);let l=i.value.get(a).content.children;for(let d of l??[]){let u=s(er(d));if(!u.ok)return u}return o.delete(a),n.add(a),{ok:!0,value:!0}};for(let a of i.value.keys()){let c=s(a);if(!c.ok)return c}return{ok:!0,value:!0}}function lm(r,e){let t=Ti(r.batches,`${e}.batches`,Ro);if(!t.ok)return t;let i=new Set;for(let n of t.value.values()){let o=n.drawIndex;if(i.has(o))return wi(`${e}.batches.drawIndex`,o.toString());i.add(o);let s=n.key,a=n.content;if(s.contentKind!==a.kind)return Qe("target_diff_disagreement",`${e}.batches`,"batch key contentKind differs from content kind")}return{ok:!0,value:!0}}function dm(r,e){let t=new Map(r),i=new Set;for(let n of e.removeResources){let o=So(n);if(i.has(o)||!t.delete(o))return Qe("duplicate_identity","$.diff.resources.removeResources",`duplicate or unknown resource ${o}`);i.add(o)}for(let n of e.upsertResources){let o=So(n);if(i.has(o))return wi("$.diff.resources.upsertResources",o);t.set(o,n)}return{ok:!0,value:t}}function Ti(r,e,t){let i=new Map;for(let n of r){let o=t(n);if(i.has(o))return wi(e,o);i.set(o,n)}return{ok:!0,value:i}}function r_(r,e){return new Map([...r].map(([t,i])=>[t,Js(i,e?.get(t)??null)]))}function n_(r,e){return new Map([...r].map(([t,i])=>{let n=e?.get(t)??null;if(!n||Zt(i,n))return[t,n??i];let o=new Map(n.batches.map(a=>[Ro(a),a])),s=Object.freeze(i.batches.map(a=>Js(a,o.get(Ro(a))??null)));return[t,Object.freeze({...i,batches:s})]}))}function o_(r,e){if(!e||Zt(r,e))return e??r;let t=new Map(e.nodes.map(n=>[er(n),n])),i=Object.freeze(r.nodes.map(n=>Js(n,t.get(er(n))??null)));return Object.freeze({...r,nodes:i})}function Js(r,e){return e!==null&&Zt(r,e)?e:r}function Zt(r,e){return cr(r,e)}function Zs(r,e){return r.size===e.size&&[...r.keys()].every(t=>e.has(t))}function So(r){return Pe(r)}function bo(r){return Pe(r.id)}function er(r){let e=r;return Pe(e.id??e)}function Ro(r){return Pe(r.key)}function wi(r,e){return Qe("duplicate_identity",r,`identity ${e} occurs more than once`)}function Qe(r,e,t){return{ok:!1,error:Object.freeze({code:r,path:e,message:t})}}function Mr(r,e,t,i={}){return{ok:!1,error:Object.freeze({code:r,path:e,message:t,...i})}}function Fc(r,e){let t={code:r.code,path:r.path,message:r.message},i=r.generationId??e.generationId,n=r.requestId??e.requestId,o=r.transactionId??e.transactionId;return i!==void 0&&(t.generationId=i),n!==void 0&&(t.requestId=n),o!==void 0&&(t.transactionId=o),{ok:!1,error:Object.freeze(t)}}var Qs=class{#e;#i=null;#n=null;#t=null;#o=new Map;#r=new Map;#s=null;constructor(e){this.#e=e}get generationId(){return this.#i}get acceptedRevision(){return this.#n}get acceptedRoot(){return this.#t}releaseRevision(e){let t=this.#n;if(t!==null&&this.#e.revisionKey(t)===this.#e.revisionKey(e))return!1;let i=this.#e.revisionKey(e);if(!this.#o.has(i))return!1;let n=[];for(let[s,a]of this.#r)a.kind==="exact"&&this.#e.revisionKey(a.targetRevision)===i&&n.push({transactionKey:s,evidence:a,fingerprint:this.#e.compactApplyPayload(a.baseRevision,a.targetRevision,a.target,a.diff)});let o=new Map(this.#o);o.delete(i),this.#o=o;for(let s of n)s.fingerprint.then(a=>{if(this.#r.get(s.transactionKey)!==s.evidence)return;let c=new Map(this.#r);c.set(s.transactionKey,Object.freeze({kind:"compact",fingerprint:a})),this.#r=c},()=>{if(this.#r.get(s.transactionKey)!==s.evidence)return;let a=new Map(this.#r);a.delete(s.transactionKey),this.#r=a;let c={code:"device_failure",path:"$",message:"Retired Apply replay evidence could not be compacted."};this.#i!==null&&(c.generationId=this.#i),this.#s=Object.freeze(c)});return!0}process(e){let t=this.#e.decode(e);return t.ok?this.processDecoded(t.value):t}processDecoded(e){return e.kind==="install"?this.#a(e):this.#s!==null?{ok:!1,error:this.#s}:e.kind==="apply"?this.#c(e):this.#l(e)}#a(e){let t=this.#e.install(e.target,e.targetRevision);if(!t.ok)return Fc(t.error,{generationId:e.generationId});let i=new Map([[this.#e.revisionKey(e.targetRevision),t.value]]);return this.#i=e.generationId,this.#n=e.targetRevision,this.#t=t.value,this.#o=i,this.#r=new Map,this.#s=null,{ok:!0,receipt:Object.freeze({kind:"installed",root:t.value})}}#c(e){let t=this.#i,i=this.#n,n=this.#t;if(t===null||i===null||n===null)return Mr("not_installed","$","A complete target must be installed before Apply.",{generationId:e.generationId,transactionId:e.transactionId});if(e.generationId!==t)return Mr("wrong_generation","$.generationId","Apply generation does not match the installed generation.",{generationId:e.generationId,transactionId:e.transactionId});let o=e.transactionKey??s_(e.transactionId),s=this.#r.get(o);if(s!==void 0){let p=s.kind==="compact"||this.#e.revisionKey(s.baseRevision)===this.#e.revisionKey(e.baseRevision)&&this.#e.revisionKey(s.targetRevision)===this.#e.revisionKey(e.targetRevision),x=s.kind==="compact"?this.#e.sameCompactedApplyPayload(s.fingerprint,e.baseRevision,e.targetRevision,e.target,e.diff):p&&this.#e.sameApplyPayload(s.target,s.diff,e.target,e.diff);return!p||!x?Mr("conflicting_transaction","$.transactionId",`Transaction ${o} was already accepted with different content.`,{generationId:e.generationId,transactionId:e.transactionId}):{ok:!0,receipt:Object.freeze({kind:"idempotent"})}}let a=this.#e.revisionKey(e.targetRevision);if(this.#o.has(a))return Mr("conflicting_transaction","$.targetRevision",`Target revision ${a} is already bound to another transaction.`,{generationId:e.generationId,transactionId:e.transactionId});let c=this.#e.revisionKey(e.baseRevision),l=this.#e.revisionKey(i);if(c!==l)return Mr("wrong_base_revision","$.baseRevision",`Apply base ${c} does not match accepted revision ${l}.`,{generationId:e.generationId,transactionId:e.transactionId});let d=this.#e.apply(n,e.target,e.diff,e.baseRevision,e.targetRevision);if(!d.ok)return Fc(d.error,{generationId:e.generationId,transactionId:e.transactionId});let u=new Map(this.#o);u.set(a,d.value);let h=new Map(this.#r);return h.set(o,Object.freeze({kind:"exact",baseRevision:e.baseRevision,targetRevision:e.targetRevision,target:e.target,diff:e.diff})),this.#n=e.targetRevision,this.#t=d.value,this.#o=u,this.#r=h,{ok:!0,receipt:Object.freeze({kind:"applied",root:d.value})}}#l(e){let t=this.#i;if(t===null)return Mr("not_installed","$","A complete target must be installed before DisplaySample.",{generationId:e.generationId,requestId:e.requestId});if(e.generationId!==t)return Mr("wrong_generation","$.generationId","DisplaySample generation does not match the installed generation.",{generationId:e.generationId,requestId:e.requestId});let i=this.#e.revisionKey(e.targetRevision),n=this.#o.get(i);if(n===void 0)return Mr("unknown_revision","$.targetRevision",`DisplaySample target revision ${i} is not retained.`,{generationId:e.generationId,requestId:e.requestId});let o=this.#e.validateSample(n,e.sample);return o.ok?{ok:!0,receipt:Object.freeze({kind:"display_sample",root:n,sample:o.value})}:Fc(o.error,{generationId:e.generationId,requestId:e.requestId})}};function s_(r){if(typeof r=="bigint"||typeof r=="number"||typeof r=="string")return`${typeof r}:${r.toString()}`;throw new Error("structured transaction identities must provide transactionKey")}var ea=class{constructor(e,t,i,n,o){this.owner=e;this.kind=t;this.generationId=i;this.revisionKey=n;this.generation=o}owner;kind;generationId;revisionKey;generation;desired=new Map;closed=!1},ta=class{#e;#i=new Map;#n=new Set;#t=!1;constructor(e){this.#e=e}beginRevisionRoot(e,t){this.#u();let i=this.#l(e);return new ea(this,"revision_root",e,t,i)}acquireRevisionRoot(e,t,i,n){return this.#d(e,"revision_root"),this.#o(e,t,i,n)}commitRevisionRoot(e){this.#d(e,"revision_root");let t=e.generation.revisions.get(e.revisionKey);if(t){if(e.closed=!0,!a_(t,e.desired))throw new Error(`retained draw revision ${e.revisionKey} cannot replace its immutable root`);return}for(let i of e.desired.values())i.references+=1;e.generation.revisions.set(e.revisionKey,new Map(e.desired)),e.closed=!0}abandonRevisionRoot(e){this.#d(e,"revision_root"),e.closed=!0}beginCandidateSample(e,t){this.#u();let i=this.#i.get(e);if(!i?.revisions.has(t))throw new Error(`retained draw revision ${t} in generation ${e} is not materialized`);return new ea(this,"candidate_sample",e,t,i)}acquireCandidateSample(e,t,i,n){return this.#d(e,"candidate_sample"),this.#o(e,t,i,n)}finishCandidateSample(e){this.#d(e,"candidate_sample"),e.closed=!0}abandonCandidateSample(e){this.#d(e,"candidate_sample"),e.closed=!0}#o(e,t,i,n){if(e.desired.has(t))return{status:"failed",message:`retained draw slot ${t} occurs more than once in one ${e.kind}`};let o=e.generation.slots.get(t);o||(o=[],e.generation.slots.set(t,o));let s=o.find(a=>this.#e(a.spec,i));if(!s){let a=n();if(a.status!=="ready")return o.length===0&&e.generation.slots.delete(t),a;s={spec:i,value:a.value,generation:e.generation,slot:t,variants:o,references:0,disposed:!1},o.push(s)}return e.desired.set(t,s),{status:"ready",lease:this.#r(s)}}updateRetainedRevisions(e,t){this.#u();let i=this.#i.get(e);if(i)for(let n of[...i.revisions.keys()])t.has(n)||this.#s(i,n)}retireGeneration(e){this.#u();let t=this.#i.get(e);if(t){this.#i.delete(e),t.retired=!0;for(let i of[...t.revisions.keys()])this.#s(t,i);t.slots.size>0&&this.#n.add(t)}}dispose(){if(this.#t)return;this.#t=!0;let e=[...this.#i.values(),...this.#n],t=[];for(let i of e)for(let n of i.slots.values())for(let o of[...n])t.push(()=>this.#c(o));t.push(()=>{for(let i of e)i.revisions.clear(),i.slots.clear();this.#i.clear(),this.#n.clear()}),rt("retained draw resource disposal",t)}#r(e){e.references+=1;let t=!1;return Object.freeze({value:e.value,release:()=>{t||(t=!0,this.#a(e))}})}#s(e,t){let i=e.revisions.get(t);if(i){e.revisions.delete(t);for(let n of i.values())this.#a(n)}}#a(e){if(!e.disposed){if(e.references-=1,e.references<0)throw new Error("retained draw resource reference count became negative");e.references===0&&this.#c(e)}}#c(e){if(e.disposed)return;e.disposed=!0;let t=e.variants.indexOf(e);t>=0&&e.variants.splice(t,1),e.variants.length===0&&e.generation.slots.delete(e.slot),e.generation.retired&&e.generation.slots.size===0&&this.#n.delete(e.generation),e.value.dispose()}#l(e){let t=this.#i.get(e);return t||(t={revisions:new Map,slots:new Map,retired:!1},this.#i.set(e,t)),t}#d(e,t){if(this.#u(),e.owner!==this)throw new Error(`retained draw ${t} belongs to another store`);if(e.kind!==t)throw new Error(`retained draw ${e.kind} cannot be used as ${t}`);if(e.closed)throw new Error(`retained draw ${t} is already closed`);if(this.#i.get(e.generationId)!==e.generation)throw new Error(`retained draw generation ${e.generationId} is retired`)}#u(){if(this.#t)throw new Error("retained draw resource store is disposed")}};function a_(r,e){return r.size===e.size&&[...r].every(([t,i])=>e.get(t)===i)}function fm(r,e){let t={positions:[],colors:[],indices:[]},i=new Set(r.voxels.map(s=>s.position.join(",")));if(!(e>=1&&r.voxels.every(s=>s.color.alpha>=1)&&i.size===r.voxels.length)){for(let s of r.voxels)for(let[a,c]of um){let l=(a+1)%3,d=(a+2)%3;hm(t,r,a,c,s.position[a]+(c>0?1:0),s.position[l],s.position[d],1,1,s.color)}return t}let o=new Map;for(let s of r.voxels)for(let[a,c]of um){let l=[...s.position];if(l[a]=l[a]+c,i.has(l.join(",")))continue;let d=s.position[a]+(c>0?1:0),{red:u,green:h,blue:p,alpha:x}=s.color,g=[a,c,d,u,h,p,x].join(","),m=o.get(g);m||(m={axis:a,side:c,plane:d,color:s.color,cells:new Map},o.set(g,m));let f=s.position[(a+1)%3],S=s.position[(a+2)%3];m.cells.set(`${f},${S}`,[f,S])}for(let s of o.values()){let a=[...s.cells.values()].sort((c,l)=>c[1]-l[1]||c[0]-l[0]);for(let[c,l]of a){if(!s.cells.has(`${c},${l}`))continue;let d=1;for(;s.cells.has(`${c+d},${l}`);)d+=1;let u=1;e:for(;;){for(let h=0;h<d;h+=1)if(!s.cells.has(`${c+h},${l+u}`))break e;u+=1}for(let h=0;h<u;h+=1)for(let p=0;p<d;p+=1)s.cells.delete(`${c+p},${l+h}`);hm(t,r,s.axis,s.side,s.plane,c,l,d,u,s.color)}}return t}var um=[[2,-1],[2,1],[0,-1],[0,1],[1,1],[1,-1]];function hm(r,e,t,i,n,o,s,a,c,l){let d=(t+1)%3,u=(t+2)%3,h=(g,m)=>{let f=[0,0,0];f[t]=n,f[d]=o+g,f[u]=s+m;let{origin:S,cellSize:_}=e.geometry;return[S[0]+f[0]*_[0],S[1]+f[1]*_[1],S[2]+f[2]*_[2]]},p;t===2&&i>0||t===0&&i<0?p=t===2?[h(a,0),h(0,0),h(a,c),h(0,c)]:[h(0,c),h(0,0),h(a,c),h(a,0)]:t===1&&i<0?p=[h(a,0),h(a,c),h(0,0),h(0,c)]:i>0?p=[h(0,0),h(0,c),h(a,0),h(a,c)]:p=[h(0,0),h(a,0),h(0,c),h(a,c)];let x=r.positions.length/3;for(let g of p)r.positions.push(...g),r.colors.push(l.red,l.green,l.blue,l.alpha);r.indices.push(x,x+1,x+2,x+2,x+1,x+3)}var Vc=`
varying vec2 copyUv;
void main() {
  copyUv = uv;
  gl_Position = vec4(position.xy, 0.0, 1.0);
}`,c_=`
uniform sampler2D sourceTexture;
varying vec2 copyUv;
void main() {
  gl_FragColor = texture2D(sourceTexture, copyUv);
  #include <colorspace_fragment>
}`,l_=`
uniform vec2 surfaceSize;
uniform float grainScale;
uniform float grainStrength;
uniform float erasureCount;
uniform vec2 erasureScale;
uniform float rulingKind;
uniform vec2 rulingSpacing;
uniform float rulingWidth;
uniform vec4 rulingColor;
varying vec2 copyUv;
float hash21(vec2 value) {
  value = fract(value * vec2(123.34, 456.21));
  value += dot(value, value + 45.32);
  return fract(value.x * value.y);
}
void main() {
  vec2 pixel = copyUv * surfaceSize;
  vec2 grainCell = floor(pixel / max(grainScale, 0.0001));
  float erased = 0.0;
  for (int index = 0; index < 64; index += 1) {
    if (float(index) >= erasureCount) break;
    float seed = float(index) + 1.0;
    vec2 center = vec2(hash21(vec2(seed, 11.0)), hash21(vec2(23.0, seed))) * surfaceSize;
    vec2 distanceValue = (pixel - center) / max(erasureScale, vec2(0.0001));
    erased = max(erased, 1.0 - smoothstep(0.65, 1.0, length(distanceValue)));
  }
  float grain = (hash21(grainCell) - 0.5) * 2.0 * grainStrength * (1.0 - erased);
  vec4 ink = vec4(grain < 0.0 ? vec3(0.0) : vec3(1.0), abs(grain));
  if (rulingKind > 0.5) {
    float horizontal = 1.0 - smoothstep(rulingWidth * 0.5, rulingWidth, min(mod(pixel.y, rulingSpacing.y), rulingSpacing.y - mod(pixel.y, rulingSpacing.y)));
    float vertical = rulingKind > 1.5
      ? 1.0 - smoothstep(rulingWidth * 0.5, rulingWidth, min(mod(pixel.x, rulingSpacing.x), rulingSpacing.x - mod(pixel.x, rulingSpacing.x)))
      : 0.0;
    float line = max(horizontal, vertical) * rulingColor.a;
    ink = mix(ink, vec4(rulingColor.rgb, line), line);
  }
  gl_FragColor = ink;
  #include <colorspace_fragment>
}`,d_=`
varying vec2 shapeUv;
void main() {
  shapeUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`,u_=`
uniform vec2 boundsSize;
uniform vec2 shapeOffset;
uniform vec2 shapeSize;
uniform vec4 fillColor;
uniform vec4 borderColor;
uniform float borderWidth;
uniform float cornerRadius;
uniform vec4 shadowColor;
uniform vec2 shadowOffset;
uniform float shadowBlur;
uniform float underlineStroke;
uniform float sketchOutlineStroke;
uniform float sketchRoughness;
uniform float sketchSeed;
uniform float batchOpacity;
varying vec2 shapeUv;
float hash21(vec2 value) {
  value = fract(value * vec2(123.34, 456.21) + sketchSeed * 0.000001);
  value += dot(value, value + 45.32);
  return fract(value.x * value.y);
}
float roundedBox(vec2 point, vec2 center, vec2 size, float radius) {
  float resolvedRadius = min(max(radius, 0.0), min(size.x, size.y) * 0.5);
  vec2 q = abs(point - center) - size * 0.5 + vec2(resolvedRadius);
  return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - resolvedRadius;
}
vec4 over(vec4 foreground, vec4 background) {
  float alpha = foreground.a + background.a * (1.0 - foreground.a);
  vec3 premultiplied = foreground.rgb * foreground.a
    + background.rgb * background.a * (1.0 - foreground.a);
  return vec4(alpha > 0.000001 ? premultiplied / alpha : vec3(0.0), alpha);
}
void main() {
  vec2 pixel = shapeUv * boundsSize;
  vec2 center = shapeOffset + shapeSize * 0.5;
  float distanceValue = roundedBox(pixel, center, shapeSize, cornerRadius);
  float shadowDistance = roundedBox(pixel - shadowOffset, center, shapeSize, cornerRadius);
  float resolvedBlur = max(shadowBlur, 0.0001);
  float shadowAlpha = (1.0 - smoothstep(-resolvedBlur * 0.25, resolvedBlur, shadowDistance))
    * shadowColor.a * (distanceValue > 0.0 ? 1.0 : 0.0);
  vec4 result = vec4(shadowColor.rgb, shadowAlpha);
  if (distanceValue <= 0.0) {
    vec4 surface = distanceValue >= -borderWidth ? borderColor : fillColor;
    result = over(surface, result);
  }
  if (sketchOutlineStroke > 0.0 || underlineStroke > 0.0) {
    float noise = (hash21(floor(gl_FragCoord.xy)) - 0.5) * sketchRoughness;
    float outline = 1.0 - smoothstep(
      max(sketchOutlineStroke, 0.0001) * (0.35 + noise * 0.25),
      max(sketchOutlineStroke, 0.0001) * (0.7 + noise * 0.25),
      abs(distanceValue)
    );
    vec2 local = pixel - shapeOffset;
    float underlineY = shapeSize.y - max(underlineStroke, 1.0) * 1.5;
    float underline = local.x >= cornerRadius && local.x <= shapeSize.x - cornerRadius
      ? 1.0 - smoothstep(
        max(underlineStroke, 0.0001) * (0.35 + noise * 0.25),
        max(underlineStroke, 0.0001) * (0.7 + noise * 0.25),
        abs(local.y - underlineY)
      )
      : 0.0;
    float sketchAlpha = max(
      sketchOutlineStroke > 0.0 ? outline : 0.0,
      underlineStroke > 0.0 && distanceValue <= 0.0 ? underline : 0.0
    ) * borderColor.a;
    result = over(vec4(borderColor.rgb, sketchAlpha), result);
  }
  result.a *= batchOpacity;
  if (result.a <= 0.000001) discard;
  gl_FragColor = result;
  #include <colorspace_fragment>
}`,h_=`
uniform sampler2D sourceTexture;
uniform vec2 sourceOrigin;
uniform vec2 sourceSize;
uniform vec2 surfaceSize;
uniform vec2 viewportOrigin;
uniform vec2 viewportSize;
uniform float pixelScale;
uniform vec4 shadowColor;
uniform vec2 shadowOffset;
uniform float shadowBlur;
uniform vec4 outlineColor;
uniform float outlineWidth;
uniform float postEffect;
uniform float postStrength;
varying vec2 copyUv;
float hash21(vec2 value) {
  value = fract(value * vec2(123.34, 456.21));
  value += dot(value, value + 45.32);
  return fract(value.x * value.y);
}
vec2 resolvedUv(vec2 value) {
  vec2 uv = sourceOrigin + value * sourceSize;
  if (pixelScale > 1.0) {
    vec2 lowSize = ceil(viewportSize / pixelScale);
    vec2 localPixel = clamp(uv * surfaceSize - viewportOrigin, vec2(0.0), viewportSize - vec2(0.5));
    vec2 lowPixel = min(floor(localPixel / viewportSize * lowSize), lowSize - vec2(1.0));
    vec2 snapped = viewportOrigin + (lowPixel + vec2(0.5)) * viewportSize / lowSize;
    uv = snapped / surfaceSize;
  }
  return uv;
}
void main() {
  vec2 uv = resolvedUv(copyUv);
  vec2 unit = 1.0 / surfaceSize;
  vec4 base = texture2D(sourceTexture, uv);
  vec2 blur = unit * max(shadowBlur, 1.0);
  float shadowAlpha = 0.25 * (
    texture2D(sourceTexture, uv - shadowOffset * unit + vec2(blur.x, 0.0)).a +
    texture2D(sourceTexture, uv - shadowOffset * unit - vec2(blur.x, 0.0)).a +
    texture2D(sourceTexture, uv - shadowOffset * unit + vec2(0.0, blur.y)).a +
    texture2D(sourceTexture, uv - shadowOffset * unit - vec2(0.0, blur.y)).a
  ) * shadowColor.a * (1.0 - base.a);
  float nearby = max(
    max(texture2D(sourceTexture, uv + vec2(outlineWidth, 0.0) * unit).a, texture2D(sourceTexture, uv - vec2(outlineWidth, 0.0) * unit).a),
    max(texture2D(sourceTexture, uv + vec2(0.0, outlineWidth) * unit).a, texture2D(sourceTexture, uv - vec2(0.0, outlineWidth) * unit).a)
  );
  float outlineAlpha = max(nearby - base.a, 0.0) * outlineColor.a;
  vec4 result = vec4(shadowColor.rgb, shadowAlpha);
  result = mix(result, vec4(outlineColor.rgb, outlineAlpha), outlineAlpha);
  result = base + result * (1.0 - base.a);
  vec2 treatmentUv = ((sourceOrigin + copyUv * sourceSize) * surfaceSize - viewportOrigin) / viewportSize;
  vec2 centered = treatmentUv * 2.0 - 1.0;
  if (postEffect > 0.5 && postEffect < 1.5) {
    float graphite = (hash21(floor(gl_FragCoord.xy)) - 0.5) * 0.18 * postStrength;
    result.rgb += graphite;
  } else if (postEffect < 2.5 && postEffect > 1.5) {
    result.rgb *= 1.0 - (0.12 + 0.08 * sin(gl_FragCoord.y * 3.14159265)) * postStrength;
  } else if (postEffect < 3.5 && postEffect > 2.5) {
    result.rgb *= 1.0 - smoothstep(0.35, 1.35, length(centered)) * postStrength;
  } else if (postEffect > 3.5) {
    result.rgb *= 1.0 + max(0.0, 1.0 - length(centered * vec2(0.72, 1.0))) * postStrength;
  }
  gl_FragColor = result;
  #include <colorspace_fragment>
}`,Tm=`
#include <common>
#include <shadowmap_pars_vertex>
attribute vec4 vertexRgba;
uniform vec3 lightDirection;
varying vec4 resolvedRgba;
varying vec3 resolvedNormal;
varying vec3 resolvedLightDirection;
void main() {
  #include <beginnormal_vertex>
  #include <defaultnormal_vertex>
  resolvedRgba = vertexRgba;
  resolvedNormal = normalize(transformedNormal);
  resolvedLightDirection = normalize((viewMatrix * vec4(lightDirection, 0.0)).xyz);
  #include <begin_vertex>
  #include <project_vertex>
  #include <worldpos_vertex>
  #include <shadowmap_vertex>
}`,Em=`
#include <common>
#include <packing>
#include <lights_pars_begin>
#include <shadowmap_pars_fragment>
#include <shadowmask_pars_fragment>
uniform float batchOpacity;
uniform float shadeEnabled;
uniform float shadowEnabled;
uniform float lightIntensity;
uniform float ambientIntensity;
uniform vec3 lightDirection;
uniform vec3 lightColor;
varying vec4 resolvedRgba;
varying vec3 resolvedNormal;
varying vec3 resolvedLightDirection;
void main() {
  float diffuse = max(dot(normalize(resolvedNormal), normalize(resolvedLightDirection)), 0.0);
  float resolvedShadow = getShadowMask();
  float directShadow = mix(1.0, resolvedShadow, shadowEnabled);
  float unshadedDenominator = ambientIntensity + lightIntensity;
  float unshadedShadow = unshadedDenominator > 0.000001
    ? (ambientIntensity + lightIntensity * directShadow) / unshadedDenominator
    : 1.0;
  vec3 shadedLight = vec3(ambientIntensity) + lightColor * lightIntensity * diffuse * directShadow;
  vec3 resolvedLight = mix(vec3(unshadedShadow), shadedLight, shadeEnabled);
  gl_FragColor = vec4(resolvedRgba.rgb * resolvedLight, resolvedRgba.a * batchOpacity);
  #include <colorspace_fragment>
}`,Bc=class{#e;constructor(e){this.#e=e}camera(e,t){let i=new Ni;return i.matrixAutoUpdate=!1,i.projectionMatrix.fromArray(e.projectionMatrix),i.projectionMatrixInverse.copy(i.projectionMatrix).invert(),i.matrixWorldInverse.fromArray(e.viewMatrix),t&&i.matrixWorldInverse.multiply(Lm),i.matrixWorld.copy(i.matrixWorldInverse).invert(),i}projectBatch(e,t=kc){let i=t.threeD?Am(e):e,n=t.threeD?Cm(t):t;switch(i.content.kind){case"pixels":return Nc(this.#i(i),i.clipPixels);case"surface_shape":return Nc(this.#n(i),i.clipPixels);case"voxels":return Nc(this.#t(i,n),i.clipPixels);case"raster_image":return this.#r(i,t.threeD===null);case"text":return this.#s(i,t.threeD===null)}}projectDecoration(e){let t=x_(e);switch(t.kind){case"lines_2d":return tr([this.#a(t)],null);case"lines_3d":return this.#c(t);case"triangles_3d":return tr([this.#l(t)],null)}}dispose(){this.#e.dispose()}#i(e){if(e.content.kind!=="pixels")throw new Error("pixel projector received non-pixel content");let{geometry:t,pixels:i,width:n,height:o}=e.content,s=[],a=[],c=[],l=t.width/n,d=t.height/o;for(let u of i){let h=t.x+u.position[0]*l,p=t.y+u.position[1]*d,x={x:h,y:p,width:l,height:d},g=t.clip?lr(x,t.clip):x;g&&I_(s,a,c,[g.x-(e.coordinateSpace==="cell"?.5:0),g.y-(e.coordinateSpace==="cell"?.5:0),0],[g.x+g.width-(e.coordinateSpace==="cell"?.5:0),g.y+g.height-(e.coordinateSpace==="cell"?.5:0),0],u.color)}return this.#o(e,s,a,c)}#n(e){if(e.content.kind!=="surface_shape")throw new Error("surface-shape projector received another primitive");let t=e.content.shape,i=Math.max(t.shadow.blurPx*2,t.sketch?.outlineStrokePx??0,t.sketch?.underlineStrokePx??0),n=Math.min(t.rect.x,t.rect.x+t.shadow.offsetXPx-i),o=Math.min(t.rect.y,t.rect.y+t.shadow.offsetYPx-i),s=Math.max(t.rect.x+t.rect.width,t.rect.x+t.rect.width+t.shadow.offsetXPx+i),a=Math.max(t.rect.y+t.rect.height,t.rect.y+t.rect.height+t.shadow.offsetYPx+i),c={x:n,y:o,width:s-n,height:a-o},l=new ci(c.width,c.height);l.translate(c.x+c.width/2,c.y+c.height/2,0);let d=new ft({uniforms:{boundsSize:{value:{x:c.width,y:c.height}},shapeOffset:{value:{x:t.rect.x-c.x,y:t.rect.y-c.y}},shapeSize:{value:{x:t.rect.width,y:t.rect.height}},fillColor:{value:On(t.fill)},borderColor:{value:On(t.border)},borderWidth:{value:t.borderWidthPx},cornerRadius:{value:t.cornerRadiusPx},shadowColor:{value:On(t.shadow.color)},shadowOffset:{value:{x:t.shadow.offsetXPx,y:t.shadow.offsetYPx}},shadowBlur:{value:t.shadow.blurPx},underlineStroke:{value:t.sketch?.underlineStrokePx??0},sketchOutlineStroke:{value:t.sketch?.outlineStrokePx??0},sketchRoughness:{value:t.sketch?.roughness??0},sketchSeed:{value:t.sketch?.seed??0},batchOpacity:{value:e.opacity}},vertexShader:d_,fragmentShader:u_,transparent:!0,depthTest:!1,depthWrite:!1,side:yt,toneMapped:!1}),u=new Xe(l,d);return wo(u,e),u}#t(e,t){if(e.content.kind!=="voxels")throw new Error("voxel projector received non-voxel content");let{positions:i,colors:n,indices:o}=fm(e.content,e.opacity);return this.#o(e,i,n,o,t)}#o(e,t,i,n,o=kc){let s=new Tt;s.setAttribute("position",new at(t,3)),s.setAttribute("vertexRgba",new at(i,4)),s.setIndex([...n]),s.computeVertexNormals();let a=o.threeD?.lighting,c=a?new D(...a.sourceDirection).normalize():new D(0,-1,-1).normalize(),l=e.opacity>=1&&i.every((h,p)=>p%4!==3||h>=1),d=new ft({uniforms:uo.merge([ie.lights,{batchOpacity:{value:e.opacity},shadeEnabled:{value:o.threeD?.shade===!0?1:0},shadowEnabled:{value:o.threeD?.shadow===!0?1:0},lightIntensity:{value:a?.intensity??0},ambientIntensity:{value:a?.ambient??1},lightDirection:{value:c},lightColor:{value:Bn(a?.color??zc)}}]),vertexShader:Tm,fragmentShader:Em,transparent:!l,depthWrite:l,side:yt,toneMapped:!1,lights:!0}),u=new Xe(s,d);return u.userData.resolvedVoxelShadowParticipant=e.content.kind==="voxels",u.castShadow=e.content.kind==="voxels"&&o.threeD?.shadow===!0,u.receiveShadow=e.content.kind==="voxels"&&o.threeD?.shadow===!0,wo(u,e),u}#r(e,t){if(e.content.kind!=="raster_image")throw new Error("raster projector received non-raster content");let i=this.#e.acquireRasterImage(e.content.resourceKey,e.content.sampling);if(i.status!=="ready")return i;try{let n=bm(i.lease.texture,Sm(e,e.content.destination),e.content.uv,zc,e.opacity,t);return wo(n,e),{status:"ready",value:tr([n],e.clipPixels,[i.lease.release])}}catch(n){return i.lease.release(),Fn(n)}}#s(e,t){if(e.content.kind!=="text")throw new Error("text projector received non-text content");let i=this.#e.acquireShapedText(e.content);if(i.status!=="ready")return i;try{if(i.lease.destination.width<=0||i.lease.destination.height<=0)return{status:"ready",value:tr([],e.clipPixels,[i.lease.release])};let n=bm(i.lease.texture,Sm(e,i.lease.destination),i.lease.uv,e.content.color,e.opacity,t);return wo(n,e),{status:"ready",value:tr([n],e.clipPixels,[i.lease.release])}}catch(n){return i.lease.release(),Fn(n)}}#a(e){let t=e.treatment??Mm;if(t.kind==="graphite")return A_({...e,treatment:t});let i=[],n=[],o=[];for(let s of e.segments){let a=s.end[0]-s.start[0],c=s.end[1]-s.start[1],l=Math.hypot(a,c);if(l===0)continue;let d=e.widthWorld/2,u=-c/l*d,h=a/l*d;Pm(i,n,o,[s.start[0]+u,s.start[1]+h,0],[s.start[0]-u,s.start[1]-h,0],[s.end[0]+u,s.end[1]+h,0],[s.end[0]-u,s.end[1]-h,0],e.color)}return E_(i,n,o,e.color.alpha<1,!1)}#c(e){let t=Rm(e.color,e.depth==="tested"),i=[],n=[],o=e.treatment??Mm,s=o.kind==="graphite"?C_({...e,treatment:o}):e.segments.map(a=>({...a,width:e.widthWorld,alpha:1}));for(let a of s){let c=new D(...a.start),l=new D(...a.end),d=l.clone().sub(c),u=d.length();if(u===0)continue;let h=new lo(a.width/2,a.width/2,u,8,1,!1);n.push(h);let p=new Xe(h,t);if(a.alpha!==1){let x=t.clone();x.opacity*=a.alpha,x.transparent=!0,p.material=x}p.position.copy(c).add(l).multiplyScalar(.5),p.quaternion.setFromUnitVectors(F_,d.normalize()),p.updateMatrix(),p.matrixAutoUpdate=!1,i.push(p)}return tr(i,null,[],[...n,t,...i.flatMap(a=>a instanceof Xe&&a.material!==t?[a.material]:[])])}#l(e){let t=[];for(let o of e.triangles)for(let s of o.points)t.push(...s);let i=new Tt;i.setAttribute("position",new at(t,3));let n=Rm(e.color,e.depth==="tested");return n.side=yt,new Xe(i,n)}},Nn=class{surface;#e;#i;#n;#t;#o;#r=new ta(cr);#s=new Nr;#a=new Ni;#c;#l;#d=new WeakMap;#u=new WeakSet;#f=0;#p=!0;#h=!1;#m;#g=null;#x=!1;constructor(e,t,i=f_,n=p_){this.surface=e,this.#n=n;let o=e.getContext("2d");if(!o)throw new Error("public OffscreenCanvas does not provide a 2D commit surface");this.#i=o,this.#m=this.#S(e,{width:Math.max(1,e.width),height:Math.max(1,e.height)}),this.#e=n(1,1),this.#t=i(this.#e),this.#t.setPixelRatio(1),this.#t.outputColorSpace=At,this.#t.shadowMap&&(this.#t.shadowMap.enabled=!0,this.#t.shadowMap.type=Jn),this.#o=new Bc(t),this.#l=new ci(2,2),this.#c=new ft({uniforms:{sourceTexture:{value:null}},vertexShader:Vc,fragmentShader:c_,depthTest:!1,depthWrite:!1,transparent:!1,toneMapped:!1}),this.#s.add(new Xe(this.#l,this.#c)),this.#a.matrixAutoUpdate=!1,this.#e.addEventListener("webglcontextlost",this.#_),this.#e.addEventListener("webglcontextrestored",this.#y)}materializeRevisionRoot(e){if(this.#x)return{status:"failed",message:"Three surface device is disposed"};if(this.#h)return{status:"failed",message:"public commit surface is unavailable"};let t=wm(e.extent);if(t)return{status:"failed",message:t};if(!this.contextState().live)return{status:"pending"};let i=this.#r.beginRevisionRoot(e.generationId.toString(),e.retainedRevisionKey),n=[],o=!1,s=(c,l,d)=>{let u=this.#r.acquireRevisionRoot(i,c,l,d);return u.status!=="ready"?u:(n.push(u.lease),null)},a=c=>(this.#r.abandonRevisionRoot(i),o=!0,ym(n),c);try{for(let c of e.viewports){for(let l of c.batches){let d=s(pm(c.key,l.key),xm(l,c.treatment),()=>{let u=this.#o.projectBatch(l,c.treatment);return u.status==="ready"?{status:"ready",value:Un(u.value)}:u});if(d)return a(d)}for(let l=0;l<c.decorations.length;l+=1){let d=c.decorations[l],u=s(mm(c.key,l),{kind:"decoration",decoration:d},()=>({status:"ready",value:Un(this.#o.projectDecoration(d))}));if(u)return a(u)}}if(e.backdrop){let c=s(gm(),{kind:"backdrop",backdrop:e.backdrop,extent:e.extent},()=>({status:"ready",value:Un(vm(e.backdrop,e.extent))}));if(c)return a(c)}return this.#r.commitRevisionRoot(i),o=!0,ym(n),{status:"ready"}}catch(c){let l=[];o||l.push(()=>this.#r.abandonRevisionRoot(i)),l.push(...n.map(d=>()=>d.release()));try{rt("revision root materialization rollback",l)}catch(d){return Fn(d)}return Fn(c)}}prepare(e){if(this.#x)return{status:"failed",message:"Three surface device is disposed"};if(this.#h)return{status:"failed",message:"public commit surface is unavailable"};let t=wm(e.extent);if(t)return{status:"failed",message:t};if(!this.contextState().live)return{status:"pending"};let i=new Ot(e.extent.width,e.extent.height,{depthBuffer:!0,stencilBuffer:!0}),n=[],o;try{o=this.#r.beginCandidateSample(e.generationId.toString(),e.retainedRevisionKey)}catch(a){return i.dispose(),Fn(a)}let s=!1;try{let a=[];for(let d of e.viewports){let u=d.treatment??kc,h=u.threeD?Cm(u):u,p=lr(d.clipPixels??d.framebuffer,U_(e.extent)),x=[...d.batches].sort(D_);for(let S=1;S<x.length;S+=1)if(x[S-1]?.drawIndex===x[S]?.drawIndex)return Mo(n),this.#r.abandonCandidateSample(o),s=!0,i.dispose(),{status:"failed",message:`viewport ${d.key} has duplicate drawIndex ${x[S]?.drawIndex.toString()}`};let g=[];for(let S of x){let _=this.#r.acquireCandidateSample(o,pm(d.key,S.key),xm(S,u),()=>{let P=this.#o.projectBatch(S,u);return P.status==="ready"?{status:"ready",value:Un(P.value)}:P});if(_.status!=="ready")return Mo(n),this.#r.abandonCandidateSample(o),s=!0,i.dispose(),_;let R=Oc(_.lease,S.clipPixels,S,u.threeD!==null);n.push(R),g.push(R)}for(let S=0;S<d.decorations.length;S+=1){let _=d.decorations[S],R=this.#r.acquireCandidateSample(o,mm(d.key,S),{kind:"decoration",decoration:_},()=>({status:"ready",value:Un(this.#o.projectDecoration(_))}));if(R.status!=="ready")return Mo(n),this.#r.abandonCandidateSample(o),s=!0,i.dispose(),R;let P=Oc(R.lease,null);n.push(P),g.push(P)}if(!p)continue;let m=u.threeD?v_(g,h):null;m&&n.push(m);let f=m?[m]:g;a.push({frame:d.framebuffer,clip:p,camera:this.#o.camera(d.camera,u.threeD!==null),draws:f,treatment:h})}if(this.#t.setRenderTarget(i),this.#t.setViewport(0,0,e.extent.width,e.extent.height),this.#t.setScissor(0,0,e.extent.width,e.extent.height),this.#t.setScissorTest(!0),this.#t.setClearColor(Bn(e.background),e.background.alpha),this.#t.clear(!0,!0,!0),e.backdrop){let d=this.#r.acquireCandidateSample(o,gm(),{kind:"backdrop",backdrop:e.backdrop,extent:e.extent},()=>({status:"ready",value:Un(vm(e.backdrop,e.extent))}));if(d.status!=="ready")return Mo(n),this.#r.abandonCandidateSample(o),s=!0,i.dispose(),d;let u=Oc(d.lease,null);n.push(u),this.#t.setViewport(0,0,e.extent.width,e.extent.height),this.#t.setScissor(0,0,e.extent.width,e.extent.height),this.#t.render(u.scene,this.#a)}for(let d of a)if(y_(d.treatment)){let u=new Ot(e.extent.width,e.extent.height,{depthBuffer:!0,stencilBuffer:!0});S_(u.texture,d.treatment);try{this.#t.setRenderTarget(u),this.#t.setScissorTest(!0),this.#t.setViewport(0,0,e.extent.width,e.extent.height),this.#t.setScissor(0,0,e.extent.width,e.extent.height),this.#t.setClearColor(0,0),this.#t.clear(!0,!0,!0),_m(this.#t,e.extent,d),this.#t.setRenderTarget(i),this.#t.setScissorTest(!0);let h=__(u.texture,d.clip,d.frame,e.extent,d.treatment);try{Im(this.#t,e.extent,d.clip),Gc(this.#t,e.extent,d.clip),this.#t.render(h.scene,this.#a)}finally{h.disposeProjection()}}finally{u.dispose()}}else this.#t.setRenderTarget(i),this.#t.setScissorTest(!0),_m(this.#t,e.extent,d);w_(n),this.#r.finishCandidateSample(o),s=!0;let c=!1,l=Object.freeze({generationId:e.generationId,requestId:e.requestId,extent:e.extent,interactionRevision:e.interactionRevision,hitMap:e.hitMap});return this.#d.set(l,Object.freeze({renderTarget:i,releaseResources:()=>{c||(c=!0,T_(n))}})),{status:"ready",candidate:l}}catch(a){return Mo(n),s||this.#r.abandonCandidateSample(o),i.dispose(),Fn(a)}}updateRetainedRevisions(e,t){this.#r.updateRetainedRevisions(e,t)}retireGeneration(e){this.#r.retireGeneration(e)}publish(e){let t=this.#R(e);if(this.#g)throw new Error("a public candidate is already awaiting its receipt boundary");let i=this.contextState();if(!i.live)throw new Error("rendering context is unavailable for publication");this.#t.setRenderTarget(null),this.#t.setSize(e.extent.width,e.extent.height,!1),this.#t.setViewport(0,0,e.extent.width,e.extent.height),this.#t.setScissor(0,0,e.extent.width,e.extent.height),this.#t.setScissorTest(!0),this.#c.uniforms.sourceTexture.value=t.renderTarget.texture,this.#t.render(this.#s,this.#a);let n=this.#t.getContext();if(n.drawingBufferWidth!==e.extent.width||n.drawingBufferHeight!==e.extent.height)throw new Error(`private drawing buffer ${n.drawingBufferWidth}x${n.drawingBufferHeight} does not match candidate ${e.extent.width}x${e.extent.height}`);let o=this.contextState();if(!o.live||o.epoch!==i.epoch)throw new Error("rendering context changed while staging publication");let s=this.#S(this.#e,e.extent),a=this.contextState();if(!a.live||a.epoch!==i.epoch)throw new Error("rendering context changed before public commit");let c=this.#m;this.#M(s,c),this.#g=Object.freeze({candidate:e,snapshot:s,previous:c})}confirmPublished(e){let t=this.#b(e);this.#m=t.snapshot,this.#g=null}rollbackPublished(e){let t=this.#b(e);try{this.#v(t.previous),this.#m=t.previous,this.#g=null}catch(i){throw this.#h=!0,this.#m=t.previous,this.#g=null,new Er(`public rollback failed: ${ia(i)}`)}}disposePrepared(e){if(this.#u.has(e))return;if(this.#g?.candidate===e)throw new Error("provisional public candidate must be confirmed or rolled back before disposal");this.#u.add(e);let t=this.#d.get(e);if(this.#d.delete(e),!t)throw new Error("prepared candidate is not owned by this Three surface device");rt("prepared Three candidate disposal",[()=>t.releaseResources(),()=>t.renderTarget.dispose()])}contextState(){if(this.#x||this.#h)return{live:!1,epoch:this.#f};let e=this.#t.getContext().isContextLost();return e&&this.#p?(this.#p=!1,this.#f+=1):!e&&!this.#p&&(this.#p=!0,this.#f+=1),{live:this.#p&&!e,epoch:this.#f}}dispose(){if(this.#x)return;this.#x=!0;let e=this.#g;this.#g=null,rt("Three surface device disposal",[...e?[()=>this.#v(e.previous)]:[],()=>this.#e.removeEventListener("webglcontextlost",this.#_),()=>this.#e.removeEventListener("webglcontextrestored",this.#y),()=>this.#l.dispose(),()=>this.#c.dispose(),()=>this.#r.dispose(),()=>this.#o.dispose(),()=>this.#t.dispose()])}#_=e=>{e.preventDefault(),this.#p&&(this.#p=!1,this.#f+=1)};#y=()=>{this.#p=!0,this.#f+=1};#R(e){if(this.#x)throw new Error("Three surface device is disposed");if(this.#h)throw new Er("public commit surface is unavailable");if(this.#u.has(e))throw new Error("prepared Three candidate has already been disposed");let t=this.#d.get(e);if(!t)throw new Error("prepared candidate is not owned by this Three surface device");return t}#S(e,t){let i=this.#n(t.width,t.height),n=i.getContext("2d");if(!n)throw new Error("private OffscreenCanvas does not provide a 2D snapshot context");return n.globalCompositeOperation="copy",n.drawImage(e,0,0,t.width,t.height),Object.freeze({canvas:i,extent:Object.freeze({...t})})}#M(e,t){try{this.#v(e)}catch(i){try{this.#v(t)}catch(n){throw this.#h=!0,new Er(`public copy and rollback both failed: ${ia(i)}; ${ia(n)}`)}throw new Hn(ia(i))}}#v(e){this.surface.width!==e.extent.width&&(this.surface.width=e.extent.width),this.surface.height!==e.extent.height&&(this.surface.height=e.extent.height),this.#i.globalCompositeOperation="copy",this.#i.drawImage(e.canvas,0,0,e.extent.width,e.extent.height)}#b(e){let t=this.#g;if(!t||t.candidate!==e)throw new Error("candidate does not own the provisional public snapshot");return t}};function f_(r){let e=new mo({canvas:r,alpha:!0,antialias:!1,depth:!0,stencil:!0,premultipliedAlpha:!1,preserveDrawingBuffer:!1});return e.autoClear=!1,e.sortObjects=!1,e}function p_(r,e){return new OffscreenCanvas(r,e)}function ia(r){return r instanceof Error?r.message:String(r)}function To(r){let e=-r[1];return Object.freeze([r[0],r[2],e===0?0:e])}function m_(r){let e=Dm.clone().multiply(new Ee().fromArray(r)).multiply(Lm).toArray();return Object.freeze(e)}function Am(r){let e=r.content.kind==="voxels"?g_(r.content):r.content;return Object.freeze({...r,cell:To(r.cell),transform:m_(r.transform),content:e})}function g_(r){let{origin:e,cellSize:t}=r.geometry;return Object.freeze({...r,geometry:Object.freeze({origin:Object.freeze([e[0],e[2],-(e[1]+r.depth*t[1])]),cellSize:Object.freeze([t[0],t[2],t[1]])}),voxels:Object.freeze(r.voxels.map(i=>Object.freeze({...i,position:Object.freeze([i.position[0],i.position[2],r.depth-1-i.position[1]])})))})}function x_(r){return r.kind==="lines_2d"?r:r.kind==="lines_3d"?Object.freeze({...r,segments:Object.freeze(r.segments.map(e=>Object.freeze({start:To(e.start),end:To(e.end)})))}):Object.freeze({...r,triangles:Object.freeze(r.triangles.map(e=>Object.freeze({points:Object.freeze(e.points.map(To))})))})}function Cm(r){let e=r.threeD;return e?Object.freeze({...r,threeD:Object.freeze({...e,lighting:Object.freeze({...e.lighting,sourceDirection:To(e.lighting.sourceDirection)})})}):r}function pm(r,e){return JSON.stringify(["batch",r,e])}function mm(r,e){return JSON.stringify(["decoration",r,e])}function gm(){return JSON.stringify(["backdrop"])}function xm(r,e){let t=r.content.kind==="voxels"?e.threeD:null;return Object.freeze({kind:"batch",coordinateSpace:r.coordinateSpace,opacity:r.opacity,content:r.content,topDownTexture:(r.content.kind==="raster_image"||r.content.kind==="text")&&e.threeD===null,voxelTreatment:t===null?null:Object.freeze({lighting:t.lighting,shade:t.shade,shadow:t.shadow})})}function Un(r){let e=!1;return Object.freeze({template:r,dispose:()=>{e||(e=!0,rt("retained projected draw disposal",[()=>r.disposeProjection(),()=>r.releaseResources()]))}})}function Oc(r,e,t,i=!1){let n=r.value.template.scene.clone(!0);if(n.matrixAutoUpdate=!1,t){let s=i?Am(t):t;for(let a of n.children)wo(a,s)}let o=!1;return{scene:n,clipPixels:e,disposeProjection:()=>{},releaseResources:()=>{o||(o=!0,n.clear(),r.release())}}}function Nc(r,e){return{status:"ready",value:tr([r],e)}}function tr(r,e,t=[],i=[]){let n=new Nr;n.matrixAutoUpdate=!1;for(let a of r)n.add(a);let o=!1,s=!1;return{scene:n,clipPixels:e,disposeProjection:()=>{o||(o=!0,rt("projected draw geometry disposal",i.length>0?i.map(a=>()=>a.dispose()):r.map(a=>()=>M_(a))))},releaseResources:()=>{s||(s=!0,rt("projected draw lease release",t.map(a=>()=>a())))}}}function v_(r,e){let t=new Nr;t.matrixAutoUpdate=!1;let i=0;for(let a of r)for(let c of[...a.scene.children])c.renderOrder=i,i+=1,c.traverse(l=>{if(l instanceof Xe){let d=l.userData.resolvedVoxelShadowParticipant===!0;l.castShadow=d&&e.threeD?.shadow===!0,l.receiveShadow=d&&e.threeD?.shadow===!0}}),t.add(c);let n=e.threeD.lighting,o=new D(...n.sourceDirection).normalize(),s=new ho(Bn(n.color),n.intensity);return s.name="resolved-directional-light",s.position.copy(o).multiplyScalar(50),s.target.position.set(0,0,0),s.castShadow=e.threeD.shadow,s.shadow.camera.left=-100,s.shadow.camera.right=100,s.shadow.camera.top=100,s.shadow.camera.bottom=-100,s.shadow.camera.near=.1,s.shadow.camera.far=200,s.shadow.mapSize.set(1024,1024),s.shadow.bias=-5e-4,t.add(s),t.add(s.target),{scene:t,clipPixels:null,disposeProjection:()=>{s.shadow.dispose()},releaseResources:()=>{t.clear()}}}function vm(r,e){let t=new ci(2,2),i=new ft({uniforms:{surfaceSize:{value:{x:e.width,y:e.height}},grainScale:{value:r.grain.scalePx},grainStrength:{value:r.grain.strength},erasureCount:{value:r.grain.erasureCount},erasureScale:{value:{x:r.grain.erasureScalePx[0],y:r.grain.erasureScalePx[1]}},rulingKind:{value:r.ruling.kind==="grid"?2:r.ruling.kind==="ruled"?1:0},rulingSpacing:{value:{x:r.ruling.spacingPx[0],y:r.ruling.spacingPx[1]}},rulingWidth:{value:r.ruling.lineWidthPx},rulingColor:{value:On(r.ruling.color)}},vertexShader:Vc,fragmentShader:l_,transparent:!0,depthTest:!1,depthWrite:!1,toneMapped:!1}),n=new Xe(t,i);return n.name="backdrop:paper",tr([n],null)}function __(r,e,t,i,n){let o=n.render.pixelSnapping?b_(e):e,s=n.threeD?.pixelate,a=new ft({uniforms:{sourceTexture:{value:r},sourceOrigin:{value:{x:o.x/i.width,y:(i.height-o.y-o.height)/i.height}},sourceSize:{value:{x:o.width/i.width,y:o.height/i.height}},surfaceSize:{value:{x:i.width,y:i.height}},viewportOrigin:{value:{x:t.x,y:i.height-t.y-t.height}},viewportSize:{value:{x:t.width,y:t.height}},pixelScale:{value:s?.enabled===!0?Math.max(1,s.scale):1},shadowColor:{value:On(n.render.shadow.color)},shadowOffset:{value:{x:n.render.shadow.offsetXPx,y:-n.render.shadow.offsetYPx}},shadowBlur:{value:n.render.shadow.blurPx},outlineColor:{value:On(n.render.outline)},outlineWidth:{value:n.render.outlineWidthPx},postEffect:{value:R_(n.render.postEffect)},postStrength:{value:n.render.postEffectStrength}},vertexShader:Vc,fragmentShader:h_,transparent:!0,depthTest:!1,depthWrite:!1,toneMapped:!1}),c=new ci(2,2),l=new Xe(c,a);return l.name=`surface-treatment:${n.render.postEffect}`,tr([l],null)}function _m(r,e,t){Im(r,e,t.frame),r.clearDepth();for(let i of t.draws){let n=i.clipPixels?lr(t.clip,i.clipPixels):t.clip;n&&(Gc(r,e,n),r.render(i.scene,t.camera))}}function y_(r){let e=r.render;return r.threeD?.pixelate.enabled===!0&&r.threeD.pixelate.scale>1||e.pixelSnapping||e.shadow.color.alpha>0&&(e.shadow.blurPx>0||e.shadow.offsetXPx!==0||e.shadow.offsetYPx!==0)||e.outline.alpha>0&&e.outlineWidthPx>0||e.postEffect!=="none"&&e.postEffectStrength>0}function S_(r,e){let t=e.threeD?.pixelate,i=t?.enabled===!0?t.smoothing?ht:nt:e.render.sampling==="nearest"?nt:ht;r.magFilter=i,r.minFilter=i,r.generateMipmaps=!1,r.needsUpdate=!0}function b_(r){let e=Math.round(r.x),t=Math.round(r.y),i=Math.round(r.x+r.width),n=Math.round(r.y+r.height);return{x:e,y:t,width:Math.max(1,i-e),height:Math.max(1,n-t)}}function R_(r){return r==="graphite"?1:r==="scanlines"?2:r==="vignette"?3:r==="stage_light"?4:0}function On(r){return{x:r.red,y:r.green,z:r.blue,w:r.alpha}}function M_(r){let e=[];r.traverse(t=>{if(t instanceof Xe)if(e.push(()=>t.geometry.dispose()),Array.isArray(t.material))for(let i of t.material)e.push(()=>i.dispose());else e.push(()=>t.material.dispose())}),rt("Three object disposal",e)}function Mo(r){rt("projected draw disposal",r.flatMap(e=>[()=>e.disposeProjection(),()=>e.releaseResources()]))}function w_(r){rt("projection disposal",r.map(e=>()=>e.disposeProjection()))}function T_(r){rt("draw resource release",r.map(e=>()=>e.releaseResources()))}function ym(r){rt("retained revision root materialization lease release",r.map(e=>()=>e.release()))}function wo(r,e){r.name=e.key;let t=new Ee().fromArray(e.transform);if(e.coordinateSpace==="cell"){let i=e.content.kind!=="voxels",n=new Ee().makeTranslation(e.cell[0]+(i?.5:0),e.cell[1]+(i?.5:0),e.cell[2]);r.matrix.multiplyMatrices(n,t)}else r.matrix.copy(t);r.matrixAutoUpdate=!1,r.matrixWorldNeedsUpdate=!0}function Sm(r,e){return r.coordinateSpace!=="cell"?e:{x:e.x-.5,y:e.y-.5,width:e.width,height:e.height}}function bm(r,e,t,i,n,o){let s=new ci(e.width,e.height);s.translate(e.x+e.width/2,e.y+e.height/2,0);let a=s.getAttribute("uv");for(let l=0;l<a.count;l+=1)a.setXY(l,t.x+a.getX(l)*t.width,t.y+(o?1-a.getY(l):a.getY(l))*t.height);let c=new bi({map:r,color:Bn(i),opacity:i.alpha*n,transparent:!0,depthWrite:!1,side:yt,toneMapped:!1});return new Xe(s,c)}function E_(r,e,t,i,n){let o=new Tt;o.setAttribute("position",new at(r,3)),o.setAttribute("color",new at(e,4)),o.setIndex([...t]);let s=new bi({vertexColors:!0,transparent:i,depthTest:n,depthWrite:n&&!i,side:yt,toneMapped:!1});return new Xe(o,s)}function A_(r){let e=r.treatment;if(e.kind!=="graphite")throw new Error("graphite projector received uniform treatment");let t=[],i=[],n=[],o=r.widthWorld/2;for(let s=0;s<r.segments.length;s+=1){let a=r.segments[s],c=a.end[0]-a.start[0],l=a.end[1]-a.start[1],d=Math.hypot(c,l);if(d===0)continue;let u=-l/d,h=c/d,p=Math.min(64,Math.max(3,Math.ceil(d*7)));for(let x=0;x<e.overdraw;x+=1){let g=(e.seed^Math.imul(s,2654435769)^Math.imul(x,2246822507))>>>0,m=(ir(g^329620769)-.5)*o*e.roughness*1.6,f=[],S=[];for(let _=0;_<=p;_+=1){let R=_/p,P=(g^Math.imul(_,668265261))>>>0,w=Math.max(Math.sin(R*Math.PI),.18),T=(ir(P)-.5)*o*e.roughness*2.8*w;f.push([a.start[0]+c*R+u*(m+T),a.start[1]+l*R+h*(m+T)]);let C=1+(ir(P^2769414579)-.5)*e.pressureVariation;S.push(Math.max(o*C,o*.28))}for(let _=0;_<p;_+=1){let R=f[_],P=f[_+1],w=P[0]-R[0],T=P[1]-R[1],C=Math.hypot(w,T);if(C===0)continue;let y=-T/C,v=w/C,I=S[_],F=S[_+1],E=t.length/3;t.push(R[0]-y*I,R[1]-v*I,0,R[0]+y*I,R[1]+v*I,0,P[0]+y*F,P[1]+v*F,0,P[0]-y*F,P[1]-v*F,0);let N=(g^Math.imul(_,374761393))>>>0,W=.5+ir(N)*.5,X=.5+ir(N^3550635116)*.5,te=ir(N^2447445413)<e.roughness*.08,V=(e.overdraw===1?1:.72)*(te?.28:1);for(let J of[W,W,X,X])i.push(r.color.red,r.color.green,r.color.blue,r.color.alpha*V*J);n.push(E,E+2,E+1,E,E+3,E+2)}}}return P_(t,i,n,!1)}function C_(r){let e=r.treatment;if(e.kind!=="graphite")return[];let t=[];for(let i=0;i<r.segments.length;i+=1){let n=r.segments[i],o=new D(...n.start),s=new D(...n.end).sub(o),a=s.length();if(a===0)continue;let c=s.clone().normalize(),l=Math.abs(c.z)<.9?new D(0,0,1):new D(0,1,0),d=c.clone().cross(l).normalize(),u=Math.min(64,Math.max(3,Math.ceil(a*7)));for(let h=0;h<e.overdraw;h+=1){let p=(e.seed^Math.imul(i,2654435769)^Math.imul(h,2246822507))>>>0,x=(ir(p^329620769)-.5)*r.widthWorld*e.roughness*.8,g=o.clone().addScaledVector(d,x);for(let m=0;m<u;m+=1){let f=(m+1)/u,S=(p^Math.imul(m+1,668265261))>>>0,_=Math.max(Math.sin(f*Math.PI),.18),R=(ir(S)-.5)*r.widthWorld*e.roughness*1.4*_,P=o.clone().addScaledVector(s,f).addScaledVector(d,x+R),w=1+(ir(S^2769414579)-.5)*e.pressureVariation;t.push({start:g.toArray(),end:P.toArray(),width:Math.max(r.widthWorld*w,r.widthWorld*.28),alpha:e.overdraw===1?1:.72}),g=P}}}return t}function ir(r){let e=r+2654435769>>>0;return e=(e^e>>>16)>>>0,e=Math.imul(e,2146121005)>>>0,e=(e^e>>>15)>>>0,e=Math.imul(e,2221713035)>>>0,e=(e^e>>>16)>>>0,e/4294967295}function P_(r,e,t,i){let n=new Tt;n.setAttribute("position",new at(r,3)),n.setAttribute("vertexRgba",new at(e,4)),n.setIndex([...t]),n.computeVertexNormals();let o=new ft({uniforms:{batchOpacity:{value:1},shadeEnabled:{value:0},shadowEnabled:{value:0},lightIntensity:{value:0},ambientIntensity:{value:1},lightDirection:{value:new D(0,-1,-1).normalize()},lightColor:{value:Bn(zc)}},vertexShader:Tm,fragmentShader:Em,transparent:!0,depthTest:i,depthWrite:!1,side:yt,toneMapped:!1});return new Xe(n,o)}function Rm(r,e){return new bi({color:Bn(r),opacity:r.alpha,transparent:r.alpha<1,depthTest:e,depthWrite:e&&r.alpha>=1,toneMapped:!1})}function I_(r,e,t,i,n,o){Pm(r,e,t,[i[0],i[1],i[2]],[n[0],i[1],i[2]],[i[0],n[1],n[2]],[n[0],n[1],n[2]],o)}function Pm(r,e,t,i,n,o,s,a){let c=r.length/3;r.push(...i,...n,...o,...s);for(let l=0;l<4;l+=1)e.push(a.red,a.green,a.blue,a.alpha);t.push(c,c+1,c+2,c+2,c+1,c+3)}function D_(r,e){return r.drawIndex<e.drawIndex?-1:r.drawIndex>e.drawIndex?1:0}var L_=Object.freeze({sampling:"smooth",pixelSnapping:!1,shadow:Object.freeze({color:Object.freeze({red:0,green:0,blue:0,alpha:0}),offsetXPx:0,offsetYPx:0,blurPx:0}),outline:Object.freeze({red:0,green:0,blue:0,alpha:0}),outlineWidthPx:0,postEffect:"none",postEffectStrength:0}),kc=Object.freeze({render:L_,threeD:null}),Mm=Object.freeze({kind:"uniform"});function Im(r,e,t){r.setViewport(t.x,e.height-t.y-t.height,t.width,t.height),Gc(r,e,t)}function Gc(r,e,t){r.setScissor(t.x,e.height-t.y-t.height,t.width,t.height)}function U_(r){return{x:0,y:0,width:r.width,height:r.height}}function wm(r){return!Number.isSafeInteger(r.width)||!Number.isSafeInteger(r.height)||r.width<=0||r.height<=0?"candidate extent must contain positive safe-integer physical pixels":null}function Bn(r){return new Me(r.red,r.green,r.blue)}function Fn(r){return{status:"failed",message:r instanceof Error?r.message:String(r)}}var zc=Object.freeze({red:1,green:1,blue:1,alpha:1}),F_=new D(0,1,0),Dm=new Ee().fromArray([1,0,0,0,0,0,-1,0,0,1,0,0,0,0,0,1]),Lm=Dm.clone().invert();function It(r){if(!r||typeof r!="object"||Array.isArray(r))throw new Error("authoring value must be an object");return r}function Dt(r){if(typeof r!="number"||!Number.isFinite(r))throw new Error("authoring value must be finite");return r}function nr(r){let e=Dt(r);if(e<=0)throw new Error("authoring extent/zoom must be positive");return e}function kn(r){let e=Dt(r);if(!Number.isSafeInteger(e)||e<0)throw new Error("authoring index must be a nonnegative integer");return e}function Eo(r){if(typeof r!="boolean")throw new Error("authoring setting must be boolean");return r}function Xc(r){if(r!=="x"&&r!=="y"&&r!=="z")throw new Error("invalid authoring axis");return r}function Wc(r){let e=It(r);return{x:Dt(e.x),y:Dt(e.y),z:Dt(e.z)}}function Um(r){if(!Array.isArray(r)||r.length!==3)throw new Error("authoring vector must have three entries");return[Dt(r[0]),Dt(r[1]),Dt(r[2])]}function Ei(r){let e=It(r);return{red:Dt(e.red),green:Dt(e.green),blue:Dt(e.blue),alpha:Dt(e.alpha)}}function Hc(r,e){if(r===null)return null;let t=It(r),i=Xc(t.axis),n=kn(t.index);if(n>=e[Ao(i)])throw new Error("authoring slice is outside the draft");return{axis:i,index:n}}function O_(r){let e=It(r);switch(e.kind){case"observe":return{kind:"observe"};case"selectSlice":return{kind:"selectSlice",axis:Xc(e.axis)};case"paint":if(e.operation!=="add"&&e.operation!=="replace"&&e.operation!=="erase")throw new Error("invalid paint operation");return{kind:"paint",operation:e.operation};case"resize":if(e.mode!=="expand"&&e.mode!=="shrink")throw new Error("invalid resize mode");return{kind:"resize",mode:e.mode};default:throw new Error("invalid authoring interaction")}}function N_(r,e){if(r===null)return null;let t=It(r);if(t.kind==="resize"){let c=Xc(t.axis);if(t.mode!=="expand"&&t.mode!=="shrink"||t.side!=="min"&&t.side!=="max")throw new Error("invalid resize highlight");return{kind:"resize",axis:c,side:t.side,mode:t.mode}}if(t.kind!=="cell"&&t.kind!=="placement")throw new Error("invalid level highlight");let i=It(t.position),n=It(i.position),o=kn(n.x),s=kn(n.y);if(o>=e[0]||s>=e[1])throw new Error("highlight outside level");if(i.kind==="grid2d")return{kind:t.kind,position:{kind:"grid2d",position:{x:o,y:s}}};if(i.kind!=="grid3d")throw new Error("invalid highlight dimension");let a=kn(n.z);if(a>=e[2])throw new Error("highlight outside level");return{kind:t.kind,position:{kind:"grid3d",position:{x:o,y:s,z:a}}}}function Om(r){let e=Pi(r,"$",["revision","size","occupied","highlight","colors","presentation","batches","decorations"]),t=Gt(e.revision,"$.revision").toString(),i=Um(e.size);for(let l of i)nr(l),kn(l);if(!Array.isArray(e.occupied)||!Array.isArray(e.batches)||!Array.isArray(e.decorations))throw new Error("authoring frame collections are invalid");let n=e.occupied.map(Um);for(let l of n)l.forEach((d,u)=>{if(kn(d),d>=i[u])throw new Error("occupied cell is outside the draft")});let o=It(e.presentation),s=It(o.surface),a=It(o.renderer);if(typeof s.surfaceId!="string"||!s.surfaceId)throw new Error("authoring surface identity is missing");let c;if(a.kind==="grid2d")c={kind:"grid2d"};else{let l=It(a.camera),d=It(a.view),u=It(a.settings);if(l.projection!=="orthographic"&&l.projection!=="perspective")throw new Error("invalid authoring projection");let h={projection:l.projection,yawDegrees:Dt(l.yawDegrees),pitchDegrees:Dt(l.pitchDegrees),rollDegrees:Dt(l.rollDegrees),zoom:nr(l.zoom)},p=d.insets===void 0?{left:0,right:0,top:0,bottom:0}:It(d.insets),x={target:Wc(d.target),insets:{left:Dt(p.left),right:Dt(p.right),top:Dt(p.top),bottom:Dt(p.bottom)}};if(Object.values(x.insets).some(g=>g<0))throw new Error("negative authoring inset");if(a.kind==="visual3d"){let g=It(a.style),m={boundsFill:Ei(g.boundsFill),boundsStroke:Ei(g.boundsStroke),voxelGridStroke:Ei(g.voxelGridStroke),activeSliceFill:Ei(g.activeSliceFill),activeSliceStroke:Ei(g.activeSliceStroke),hoverSliceFill:Ei(g.hoverSliceFill),hoverSliceStroke:Ei(g.hoverSliceStroke),selectionFill:Ei(g.selectionFill),selectionStroke:Ei(g.selectionStroke)},f=a.selectionBox===null?null:It(a.selectionBox),S=f?{min:Wc(f.min),max:Wc(f.max)}:null;if(S){for(let[_,R]of["x","y","z"].entries())if(S.min[R]<0||S.max[R]<S.min[R]||S.max[R]>=i[_])throw new Error("selection is outside the draft")}c={kind:"visual3d",camera:h,view:x,activeSlice:Hc(a.activeSlice,i),hoverSlice:Hc(a.hoverSlice,i),selectionBox:S,settings:{voxelGridVisible:Eo(u.voxelGridVisible),boundsVisible:Eo(u.boundsVisible)},style:m}}else if(a.kind==="grid3d"){let g=a.viewportFraming==null?null:It(a.viewportFraming);c={kind:"grid3d",sliceZ:a.sliceZ==null?null:Hc({axis:"z",index:a.sliceZ},i).index,camera:h,view:x,viewportFraming:g?{width:nr(g.width),depth:nr(g.depth),height:g.height==null?null:nr(g.height)}:null,settings:{gridVisible:Eo(u.gridVisible),occupiedCellFrames:Eo(u.occupiedCellFrames),stageFrame:Eo(u.stageFrame)}}}else throw new Error("unsupported authoring strategy")}return{revision:t,size:i,occupied:n,highlight:N_(e.highlight,i),colors:e.colors===null?null:{accent:Ei(It(e.colors).accent),grid:Ei(It(e.colors).grid)},surfaceId:s.surfaceId,interaction:O_(s.interaction),strategy:c,batches:e.batches.map(Ll),decorations:e.decorations.map(Ul)}}function Nm(r){let e=Pi(r,"$.metrics",["width","height","cssWidth","cssHeight"]);return{width:nr(e.width),height:nr(e.height),cssWidth:nr(e.cssWidth),cssHeight:nr(e.cssHeight)}}function Fm(r){return r.elements.slice()}function Ao(r){return r==="x"?0:r==="y"?1:2}var B_=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];function Bm(r,e){let t=r.strategy;if(t.kind==="grid2d"){let E=Math.min(e.cssWidth/r.size[0],e.cssHeight/r.size[1]),N={x:(e.cssWidth-r.size[0]*E)/2,y:(e.cssHeight-r.size[1]*E)/2,width:r.size[0]*E,height:r.size[1]*E},W=e.cssWidth/E,X=e.cssHeight/E,te=new Ee().makeOrthographic(-(W-r.size[0])/2,(W+r.size[0])/2,-(X-r.size[1])/2,(X+r.size[1])/2,-1e3,1e3);return{frame:r,metrics:e,camera:{kind:"orthographic",projectionMatrix:Fm(te),viewMatrix:B_},framebuffer:{x:0,y:0,width:e.width,height:e.height},worldPerPixel:1/(E*e.width/e.cssWidth),gridRect:N}}let{insets:i,target:n}=t.view,o=e.cssWidth-i.left-i.right,s=e.cssHeight-i.top-i.bottom;if(o<=0||s<=0)throw new Error("authoring insets leave no drawable area");let a=t.camera.yawDegrees*Math.PI/180*(t.kind==="grid3d"?-1:1),c=t.camera.pitchDegrees*Math.PI/180,l=t.camera.rollDegrees*Math.PI/180,d=new D(Math.cos(a),-Math.sin(a),0),u=new D(-Math.sin(a)*Math.sin(c),-Math.cos(a)*Math.sin(c),-Math.cos(c)),h=d.clone().multiplyScalar(Math.cos(l)).addScaledVector(u,-Math.sin(l)),p=d.clone().multiplyScalar(-Math.sin(l)).addScaledVector(u,-Math.cos(l)),x=new D().crossVectors(h,p).normalize(),g=new D(n.x,n.y,n.z),m=o/s,f=t.kind==="grid3d"&&t.viewportFraming?[t.viewportFraming.width,t.viewportFraming.depth,t.viewportFraming.height??r.size[2]]:r.size,S=t.kind==="grid3d"&&t.viewportFraming?g:new D((r.size[0]-1)/2,(r.size[1]-1)/2,(r.size[2]-1)/2),_=km(f.map((E,N)=>S.getComponent(N)-E/2),f.map((E,N)=>S.getComponent(N)+E/2)).map(E=>new D(...E).sub(g)),R=Math.max(.01,..._.map(E=>Math.max(Math.abs(E.dot(p)),Math.abs(E.dot(h))/m)))/t.camera.zoom,P=Math.hypot(...r.size),w=Math.max(P*4,1),T=Math.max(1e3,P*100),C=t.camera.projection==="perspective"?Math.max(.1,..._.map(E=>E.dot(x)+Math.max(Math.abs(E.dot(p)),Math.abs(E.dot(h))/m)/Math.tan(Math.PI/8)))/t.camera.zoom:w,y=[h.x,p.x,x.x,0,h.y,p.y,x.y,0,h.z,p.z,x.z,0,-h.dot(g),-p.dot(g),-x.dot(g)-C,1],v=t.camera.projection==="perspective"?new Ee().makePerspective(-.1*Math.tan(Math.PI/8)*m,.1*Math.tan(Math.PI/8)*m,.1*Math.tan(Math.PI/8),-.1*Math.tan(Math.PI/8),.1,T):new Ee().makeOrthographic(-R*m,R*m,R,-R,.1,T),I=e.width/e.cssWidth,F=e.height/e.cssHeight;return{frame:r,metrics:e,camera:{kind:t.camera.projection,projectionMatrix:Fm(v),viewMatrix:y},framebuffer:{x:i.left*I,y:i.top*F,width:o*I,height:s*F},worldPerPixel:2*R/(s*F),gridRect:null}}function km(r,e){return[[r[0],r[1],r[2]],[e[0],r[1],r[2]],[e[0],e[1],r[2]],[r[0],e[1],r[2]],[r[0],r[1],e[2]],[e[0],r[1],e[2]],[e[0],e[1],e[2]],[r[0],e[1],e[2]]]}var k_=[[0,1],[1,2],[2,3],[3,0],[4,5],[5,6],[6,7],[7,4],[0,4],[1,5],[2,6],[3,7]],z_=[[0,1,2,3],[4,5,6,7],[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7]];function rr(r,e,t,i,n,o="tested"){let s=km(r,e);return[{kind:"triangles_3d",triangles:z_.flatMap(([a,c,l,d])=>[{points:[s[a],s[c],s[l]]},{points:[s[a],s[l],s[d]]}]),color:t,depth:o},{kind:"lines_3d",segments:k_.map(([a,c])=>({start:s[a],end:s[c]})),color:i,widthWorld:n,depth:o,treatment:{kind:"uniform"}}]}var or={red:0,green:0,blue:0,alpha:0},qc={render:{sampling:"nearest",pixelSnapping:!1,shadow:{color:or,offsetXPx:0,offsetYPx:0,blurPx:0},outline:or,outlineWidthPx:0,postEffect:"none",postEffectStrength:0},threeD:{lighting:{intensity:1,ambient:.65,sourceDirection:[-.3,-.5,1],color:{red:1,green:1,blue:1,alpha:1}},shade:!0,shadow:!1,pixelate:{enabled:!1,scale:1,smoothing:!1}}};function V_(r){let{frame:e,worldPerPixel:t}=r,i=e.highlight;if(!i||i.kind==="slice")return[];if(!e.colors)throw new Error("level highlight requires resolved theme colors");let n=e.colors.accent;if(e.strategy.kind==="grid2d"){let c;if(i.kind==="resize"){let l=i.side==="min"?0:e.size[0],d=i.side==="min"?0:e.size[1];c=i.axis==="x"?[[l,0],[l,e.size[1]]]:[[0,d],[e.size[0],d]]}else{let{x:l,y:d}=i.position.position;c=[[l,d],[l+1,d],[l+1,d+1],[l,d+1],[l,d]]}return[{kind:"lines_2d",segments:c.slice(1).map((l,d)=>({start:c[d],end:l})),color:n,widthWorld:2*t,treatment:{kind:"uniform"}}]}if(i.kind==="resize"){let c=$c(e.size,i.mode).find(l=>l.axis===i.axis&&l.side===i.side);return c?rr(c.min,c.max,or,n,2*t,"overlay"):[]}if(i.position.kind!=="grid3d")throw new Error("highlight dimension differs from frame");let{x:o,y:s,z:a}=i.position.position;return rr([o-.5,s-.5,a-.5],[o+.5,s+.5,a+.5],or,n,2*t,"overlay")}function zm(r){let{frame:e,worldPerPixel:t}=r,i=e.strategy,n=V_(r);if(i.kind==="grid2d")return n;let o=e.size.map(a=>a-.5);if(i.kind==="grid3d"&&!e.colors)throw new Error("level decorations require resolved theme colors");let s=e.colors?.grid??or;if(e.interaction.kind==="resize")for(let a of $c(e.size,e.interaction.mode))n.push(...rr(a.min,a.max,or,s,t));if(i.kind==="visual3d"){if(i.settings.boundsVisible&&n.push(...rr([-.5,-.5,-.5],o,i.style.boundsFill,i.style.boundsStroke,t)),i.settings.voxelGridVisible)for(let a of e.occupied)n.push(...rr(a.map(c=>c-.5),a.map(c=>c+.5),or,i.style.voxelGridStroke,t));for(let[a,c,l]of[[i.hoverSlice,i.style.hoverSliceFill,i.style.hoverSliceStroke],[i.activeSlice,i.style.activeSliceFill,i.style.activeSliceStroke]]){if(!a)continue;let d=[-.5,-.5,-.5],u=[...o];d[Ao(a.axis)]=a.index-.5,u[Ao(a.axis)]=a.index+.5,n.push(...rr(d,u,c,l,t))}if(i.selectionBox){let{min:a,max:c}=i.selectionBox;n.push(...rr([a.x-.5,a.y-.5,a.z-.5],[c.x+.5,c.y+.5,c.z+.5],i.style.selectionFill,i.style.selectionStroke,t*1.5,"overlay"))}}else{if(i.settings.stageFrame&&n.push(...rr([-.5,-.5,-.5],o,or,s,t)),i.settings.occupiedCellFrames)for(let a of e.occupied)n.push(...rr(a.map(c=>c-.5),a.map(c=>c+.5),or,s,t));if(i.settings.gridVisible){let a=[];for(let c=0;c<=e.size[0];c++)a.push({start:[c-.5,-.5,i.sliceZ===null?-.5:i.sliceZ+.5],end:[c-.5,o[1],i.sliceZ===null?-.5:i.sliceZ+.5]});for(let c=0;c<=e.size[1];c++)a.push({start:[-.5,c-.5,i.sliceZ===null?-.5:i.sliceZ+.5],end:[o[0],c-.5,i.sliceZ===null?-.5:i.sliceZ+.5]});n.push({kind:"lines_3d",segments:a,color:s,widthWorld:t,depth:"tested",treatment:{kind:"uniform"}})}}return n}function $c(r,e){let t=[];for(let[i,n]of["x","y","z"].entries())if(!(e==="shrink"&&r[i]<=1))for(let o of["min","max"]){let s=[-.5,-.5,-.5],a=r.map(d=>d-.5),c=e==="expand"?1:0,l=o==="min"?-.5-c:r[i]-.5+c;s[i]=l-.12,a[i]=l+.12,t.push({min:s,max:a,axis:n,side:o})}return t}function jc(r,e,t,i){let n=-1/0,o=1/0;for(let s=0;s<3;s++){let a=r.getComponent(s),c=e.getComponent(s);if(Math.abs(c)<1e-10){if(a<t[s]||a>i[s])return null;continue}let l=(t[s]-a)/c,d=(i[s]-a)/c;if(n=Math.max(n,Math.min(l,d)),o=Math.min(o,Math.max(l,d)),n>o)return null}return o<0?null:Math.max(n,0)}function Vm(r,e,t,i){let{frame:n,metrics:o}=r,s=n.interaction;if(s.kind==="observe")return null;if(n.strategy.kind==="grid2d"){let f=r.gridRect,S=Math.floor((e-f.x)/f.width*n.size[0]),_=Math.floor((t-f.y)/f.height*n.size[1]);if(s.kind==="resize"){let T=[["x","min",Math.abs(e-f.x)],["x","max",Math.abs(e-f.x-f.width)],["y","min",Math.abs(t-f.y)],["y","max",Math.abs(t-f.y-f.height)]].filter(([y])=>s.mode==="expand"||n.size[Ao(y)]>1);T.sort((y,v)=>y[2]-v[2]);let C=T[0];return C&&C[2]<=12&&e>=f.x-12&&e<=f.x+f.width+12&&t>=f.y-12&&t<=f.y+f.height+12?{kind:"resize",mode:s.mode,axis:C[0],side:C[1]}:null}if(S<0||_<0||S>=n.size[0]||_>=n.size[1]||s.kind!=="paint")return null;let R=n.occupied.some(w=>w[0]===S&&w[1]===_),P=i??s.operation;return R&&P==="add"||!R&&P==="erase"?null:{kind:R?"cell":"placement",position:{kind:"grid2d",position:{x:S,y:_}}}}let a=r.framebuffer,c=e*o.width/o.cssWidth,l=t*o.height/o.cssHeight;if(c<a.x||l<a.y||c>a.x+a.width||l>a.y+a.height)return null;let d=new Ee().fromArray(r.camera.projectionMatrix).multiply(new Ee().fromArray(r.camera.viewMatrix)).invert(),u=2*(c-a.x)/a.width-1,h=1-2*(l-a.y)/a.height,p=new D(u,h,-1).applyMatrix4(d),x=new D(u,h,1).applyMatrix4(d).sub(p).normalize(),g=null,m=1/0;for(let f of n.occupied){let S=jc(p,x,f.map(_=>_-.5),f.map(_=>_+.5));S!==null&&S<m&&(m=S,g=f)}if(s.kind==="selectSlice"){let f=Ao(s.axis);if(g)return{kind:"slice",axis:s.axis,index:g[f]};let S=jc(p,x,[-.5,-.5,-.5],n.size.map(R=>R-.5));if(S===null)return null;let _=p.clone().addScaledVector(x,S);return{kind:"slice",axis:s.axis,index:Math.max(0,Math.min(n.size[f]-1,Math.floor(_.getComponent(f)+.5)))}}if(s.kind==="resize"){let f=null,S=1/0;for(let _ of $c(n.size,s.mode)){let R=jc(p,x,_.min,_.max);R!==null&&R<S&&(S=R,f={kind:"resize",mode:s.mode,axis:_.axis,side:_.side})}return f}if(s.kind==="paint"&&n.strategy.kind==="grid3d"){let f=i??s.operation,S=T=>n.occupied.some(C=>C.every((y,v)=>y===T[v])),_=T=>T.every((C,y)=>C>=0&&C<n.size[y]),R=(T,C)=>({kind:T,position:{kind:"grid3d",position:{x:C[0],y:C[1],z:C[2]}}}),P=(T,C)=>{if(Math.abs(x.z)<1e-9)return null;let y=(C-p.z)/x.z;if(y<0)return null;let v=p.clone().addScaledVector(x,y),I=[Math.floor(v.x+.5),Math.floor(v.y+.5),T];return _(I)?I:null};if(n.strategy.sliceZ!==null){let T=P(n.strategy.sliceZ,n.strategy.sliceZ+.5);if(!T)return null;let C=S(T);return C&&f==="add"||!C&&f==="erase"?null:R(C?"cell":"placement",T)}if(g&&f!=="add")return R("cell",g);if(f==="erase")return null;if(g){let T=p.clone().addScaledVector(x,m),C=[0,1,2].sort((v,I)=>Math.abs(T.getComponent(I)-g[I])-Math.abs(T.getComponent(v)-g[v]))[0],y=[...g];if(y[C]=y[C]+Math.sign(T.getComponent(C)-g[C]),_(y)&&!S(y))return R("placement",y)}let w=P(0,-.5);if(w&&!S(w))return R("placement",w)}return null}var Yc=class{#e;#i;#n;#t=null;#o=null;#r=null;#s=!1;constructor(e,t,i){this.#e=new Cn;let n=this.#e.installResources(t);if(n.status==="rejected")throw new Error(n.rejection.message);this.#i=new Hr(this.#e,new Ln),this.#n=i?i(e,this.#e):new Nn(e,this.#e)}prepare(e,t){if(this.#s)throw new Error("authoring publication is awaiting its receipt");let i=Om(e),n=Nm(t);if(this.#t){if(this.#t.geometry.frame.revision!==i.revision)throw new Error("authoring candidate must complete before replacement")}else{let c=Bm(i,n),l=i.batches.map(h=>{let p=this.#i.projectResolvedBatch("1",h);if(p.status==="failed")throw new Error(p.message);return p.batch}),d=[...i.decorations.map(h=>Uc(h,c.worldPerPixel)),...zm(c)],u={generationId:1n,requestId:BigInt(i.revision),retainedRevisionKey:i.revision,interactionRevision:BigInt(i.revision),extent:{width:n.width,height:n.height},background:{red:0,green:0,blue:0,alpha:0},backdrop:null,viewports:[{key:"authoring",framebuffer:c.framebuffer,clipPixels:c.framebuffer,camera:c.camera,treatment:i.strategy.kind==="grid2d"?{...qc,threeD:null}:qc,batches:l,decorations:d}],hitMap:{regions:[]}};this.#t={geometry:c,candidate:u}}if(this.#o)return!0;let o=this.#t.candidate,s=this.#n.materializeRevisionRoot(o);if(s.status==="failed")throw new Error(s.message);if(s.status==="pending")return!1;let a=this.#n.prepare(o);if(a.status==="failed")throw new Error(a.message);return a.status==="pending"?!1:(this.#o=a.candidate,!0)}present(){if(!this.#o||this.#s)throw new Error("authoring candidate is not ready to publish");this.#n.publish(this.#o),this.#s=!0}commit(){if(!this.#s||!this.#o||!this.#t)throw new Error("authoring commit requires the exposed candidate");this.#n.confirmPublished(this.#o),this.#r=this.#t.geometry,this.#n.disposePrepared(this.#o),this.#n.updateRetainedRevisions("1",new Set([this.#t.candidate.retainedRevisionKey])),this.#o=null,this.#t=null,this.#s=!1}reject(){this.#s&&this.#o&&this.#n.rollbackPublished(this.#o),this.#o&&this.#n.disposePrepared(this.#o),this.#o=null,this.#t=null,this.#s=!1,this.#n.updateRetainedRevisions("1",new Set(this.#r?[this.#r.frame.revision]:[]))}hit(e){let t=Pi(e,"$.pointer",["x","y","gesture","operation"]);if(typeof t.x!="number"||!Number.isFinite(t.x)||typeof t.y!="number"||!Number.isFinite(t.y))throw new Error("authoring pointer coordinates must be finite");if(!["move","press","release","leave"].includes(String(t.gesture)))throw new Error("invalid authoring pointer gesture");if(t.operation!==null&&t.operation!=="add"&&t.operation!=="replace"&&t.operation!=="erase")throw new Error("invalid authoring pointer operation");return!this.#r||t.gesture==="leave"?null:Vm(this.#r,t.x,t.y,t.operation??void 0)}dispose(){this.reject(),this.#n.dispose(),this.#e.dispose(),this.#r=null}};function G_(r,e){return new Yc(r,e)}var Jc=class{#e;#i=new Map;#n;#t;#o=new Map;#r=new Map;#s=[];#a=0;#c=null;#l=null;#d=null;#u=!1;constructor(e,t={}){let i=t.browserSurfaceDeviceFactory?t.browserSurfaceDeviceFactory(e):W_(e,t);this.#e=i.resources,this.#t=i.device,this.#n=new Io(this.#t,t.scheduler??new Do)}installResources(e){return this.#e.installResources(e)}install(e){let t;try{t=El(e)}catch(c){return wr(c)}let i=t.generationId.toString();if(!this.#e.hasGeneration(i))return Ye("not_installed","$.generationId",`resources for generation ${i} are not installed`,{generationId:i});let n=Gm(this.#e,i,t.target);if(n)return{status:"rejected",rejection:n};if(this.#i.has(i))return Ye("duplicate_identity","$.generationId",`generation ${i} is already installed`,{generationId:i});let o=new Ln,s={generationId:i,ordinal:this.#a,store:new Qs(new Ks),interaction:o,projector:new Hr(this.#e,o),requestFingerprints:new Map,consumedRequests:new Set,revisionValues:new Map([[mi(t.targetRevision),t.targetRevision]]),materializedRevisionRoots:new Set,committedReceipts:[],commitFailures:[]},a=s.store.processDecoded({kind:"install",generationId:t.generationId,targetRevision:t.targetRevision,target:t.target});return a.ok?(this.#a+=1,this.#i.set(i,s),zn()):Zc(a.error)}apply(e){let t;try{t=Al(e)}catch(a){return wr(a)}if(mi(t.transactionId)!==mi(t.targetRevision))return Ye("target_diff_disagreement","$.transactionId","transactionId must equal targetRevision",{generationId:t.generationId.toString(),transactionId:t.transactionId});let i=t.generationId.toString(),n=this.#i.get(i);if(!n)return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i});let o=Gm(this.#e,t.generationId.toString(),t.target);if(o)return{status:"rejected",rejection:o};let s=n.store.processDecoded({kind:"apply",generationId:t.generationId,transactionId:t.transactionId,transactionKey:mi(t.transactionId),baseRevision:t.baseRevision,targetRevision:t.targetRevision,target:t.target,diff:t.diff});return s.ok?(n.revisionValues.set(mi(t.targetRevision),t.targetRevision),zn()):Zc(s.error)}displaySample(e){let t;try{t=Cl(e)}catch(m){return wr(m)}let i=t.generationId.toString(),n=this.#i.get(i);if(!n)return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i,requestId:t.requestId.toString()});let o=t.requestId.toString(),s=ra(i,o),a=Lo(t.sample,"display-sample:v1"),c=n.requestFingerprints.get(o);if(c!==void 0&&c!==a)return Ye("conflicting_transaction","$.requestId",`request ${o} was already observed with different content`,{generationId:t.generationId.toString(),requestId:o});if(this.#o.get(s))return Object.freeze({status:"ready",candidateId:o});if(n.consumedRequests.has(o))return Ye("duplicate_identity","$.requestId",`request ${o} has already produced a consumed candidate`,{generationId:t.generationId.toString(),requestId:o});let d=n.store.processDecoded({kind:"display_sample",generationId:t.generationId,requestId:t.requestId,targetRevision:t.targetRevision,sample:Object.freeze({wire:t.sample})});if(!d.ok)return Zc(d.error);if(d.receipt.kind!=="display_sample"||!d.receipt.sample.composed)return Ye("invalid_schema","$","retained sample did not compose a complete scene",{generationId:t.generationId.toString(),requestId:o});let u=X_(this.#e,t.generationId.toString(),d.receipt.root,d.receipt.sample.composed);if(u)return{status:"rejected",rejection:$_(u,o)};n.requestFingerprints.set(o,a);let h=mi(t.targetRevision);if(!n.materializedRevisionRoots.has(h)){let f=new Hr(this.#e,n.interaction.fork()).projectRevisionRoot(i,t.requestId,H_(d.receipt.root,d.receipt.sample.composed));if(f.status==="pending")return Object.freeze({status:"pending"});if(f.status==="failed")return Ye(na(f.code),f.path,f.message,{generationId:i,requestId:o});let S=this.#t.materializeRevisionRoot(j_(h,f.candidate));if(S.status==="pending")return Object.freeze({status:"pending"});if(S.status==="failed")return Ye("invalid_schema","$",S.message,{generationId:i,requestId:o});n.materializedRevisionRoots.add(h)}let p=n.projector.project(t.generationId.toString(),t.requestId,d.receipt.sample.composed);if(p.status==="pending")return Object.freeze({status:"pending"});if(p.status==="failed")return Ye(na(p.code),p.path,p.message,{generationId:t.generationId.toString(),requestId:o});let x=Object.freeze({...p.candidate,retainedRevisionKey:h}),g=this.#n.displaySample(x);return g.status==="pending"?Object.freeze({status:"pending"}):g.status==="failed"?Ye("invalid_schema","$",g.message,{generationId:t.generationId.toString(),requestId:o}):(this.#o.set(s,Object.freeze({generationId:i,candidate:g.candidate,layout:p.layout})),Object.freeze({status:"ready",candidateId:o}))}beginCommit(e){let t;try{t=Il(e)}catch(l){return wr(l)}let i=t.generationId.toString(),n=t.candidateId.toString(),o=ra(i,n),s=this.#i.get(i);if(!s)return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i,requestId:n});let a=this.#o.get(o);if(!a)return Ye("unknown_reference","$.candidateId",`unknown prepared candidate ${n} in generation ${i}`,{generationId:i,requestId:n});this.#o.delete(o),s.consumedRequests.add(n),this.#r.set(o,a),this.#s.push(o);let c=this.#h();return c?.key===o?(this.#s=this.#s.filter(l=>l!==o),this.#r.delete(o),s.consumedRequests.delete(n),this.#o.set(o,a),Ye("invalid_schema","$.candidateId",c.message,{generationId:i,requestId:n})):zn()}pollCommitted(e){let i=ko(e).generationId.toString(),n=this.#i.get(i);if(!n)throw new Error(`generation ${i} is not installed`);this.#p();let o=n.commitFailures.shift();if(o)return this.#h(),Object.freeze({status:"failed",candidateId:o.candidateId,rejection:sr("device_failure","$",o.message,{generationId:i,requestId:o.candidateId})});let s=n.committedReceipts.shift();return s?(n.interaction.activateLayout(s.layout),this.#c=Object.freeze({generationId:i,layoutId:s.response.candidateId,layout:s.layout}),this.#g(n.ordinal),this.#h(),s.response):(this.#h(),Object.freeze({status:"none"}))}updateRetainedRevisions(e){let t;try{t=Dl(e)}catch(a){return wr(a)}let i=t.generationId.toString(),n=this.#i.get(i);if(!n)return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i});let o=new Set;for(let a of t.revisions){let c=mi(a);if(o.has(c))return Ye("duplicate_identity","$.revisions",`duplicate retained revision ${c}`,{generationId:i});if(!n.revisionValues.has(c))return Ye("unknown_revision","$.revisions",`revision ${c} is not retained`,{generationId:i});o.add(c)}let s=n.store.acceptedRevision;if(s!==null&&!o.has(mi(s)))return Ye("unknown_reference","$.revisions","accepted revision must remain retained",{generationId:i});for(let[a,c]of[...n.revisionValues])o.has(a)||(n.store.releaseRevision(c),n.revisionValues.delete(a),n.materializedRevisionRoots.delete(a));return this.#t.updateRetainedRevisions(i,o),zn()}retireGeneration(e){let t;try{t=ko(e)}catch(a){return wr(a)}let i=t.generationId.toString();if(!this.#i.has(i))return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i});if(this.#p(),this.#c?.generationId===i)return Ye("unknown_reference","$.generationId","active receipt-backed generation cannot be retired",{generationId:i});let n=this.#i.get(i);if(n.committedReceipts.length>0||n.commitFailures.length>0)return Ye("unknown_reference","$.generationId","generation has an undelivered publication outcome awaiting pollCommitted",{generationId:i});let o=this.#l===i?this.#d:null,s=o?this.#r.get(o):void 0;return s&&this.#n.isSubmitted(s.candidate)?Ye("unknown_reference","$.generationId","generation has a non-supersedable public copy awaiting its receipt boundary",{generationId:i}):(this.#f(i),this.#t.retireGeneration(i),this.#i.delete(i),this.#e.retireGeneration(i),this.#h(),zn())}stopGeneration(e){let t;try{t=ko(e)}catch(o){return wr(o)}let i=t.generationId.toString();if(!this.#i.has(i))return Ye("not_installed","$.generationId",`generation ${i} is not installed`,{generationId:i});let n=new Set;for(let[o,s]of[...this.#o])s.generationId===i&&(this.#o.delete(o),n.add(s.candidate));for(let[o,s]of[...this.#r])s.generationId===i&&(this.#r.delete(o),n.add(s.candidate));return this.#i.delete(i),this.#s=this.#s.filter(o=>this.#r.has(o)),this.#c?.generationId===i&&(this.#c=null),this.#l===i&&(this.#l=null,this.#d=null),rt(`browser generation ${i} stop`,[()=>this.#n.resetGeneration(t.generationId),...[...n].map(o=>()=>this.#n.discard(o)),()=>this.#t.retireGeneration(i),()=>{this.#e.retireGeneration(i)},()=>{let o=this.#h();if(o)throw new Error(o.message)}]),zn()}pointerInput(e){let t;try{t=Pl(e)}catch(a){return wr(a)}let i=t.generationId.toString(),n=this.#i.get(i);if(!n)return Ye("wrong_generation","$.generationId","pointer input generation does not match the installed target",{generationId:i});let o=t.layoutId.toString();if(!this.#c||this.#c.generationId!==i||this.#c.layoutId!==o)return Ye("unknown_reference","$.layoutId",`layout ${o} is not the last receipt-backed layout`,{generationId:i});this.#p();let s=n.interaction.apply(this.#c.layout,t.input);return s.ok?(s.changed&&this.#f(i),Object.freeze({status:"applied",changed:s.changed,interactionRevision:s.interactionRevision.toString()})):Ye(na(s.code),s.path,s.message,{generationId:i})}dispose(){if(this.#u)return;this.#u=!0;let e=[...this.#i.keys()];this.#i.clear(),rt("browser renderer disposal",[...e.map(t=>(()=>this.#f(t))),()=>this.#n.dispose(),()=>this.#e.dispose(),()=>{this.#o.clear(),this.#r.clear(),this.#s=[],this.#c=null,this.#l=null,this.#d=null}])}#f(e){let t=this.#i.get(e),i=[];for(let[s,a]of[...this.#o])a.generationId===e&&(t?.consumedRequests.add(a.candidate.requestId.toString()),this.#o.delete(s),i.push(()=>this.#n.discard(a.candidate)));let n=this.#l===e?this.#d:null,o=null;if(n){let s=this.#r.get(n);s&&this.#n.isSubmitted(s.candidate)?o=n:(this.#r.delete(n),this.#l=null,this.#d=null,i.push(()=>this.#n.supersedePending()))}for(let[s,a]of[...this.#r])a.generationId===e&&s!==o&&(this.#r.delete(s),i.push(()=>this.#n.discard(a.candidate)));this.#s=this.#s.filter(s=>this.#r.has(s)),i.push(()=>{this.#h()}),rt(`generation ${e} candidate invalidation`,i)}#p(){for(;;){let e=this.#n.pollCommitted();if(!e)break;let t=e.generationId.toString(),i=e.candidateId.toString(),n=ra(t,i),o=this.#r.get(n);this.#r.delete(n),this.#d===n&&(this.#l=null,this.#d=null);let s=this.#i.get(t);!s||!o||s.committedReceipts.push(Object.freeze({response:q_(e),layout:o.layout}))}for(;;){let e=this.#n.pollFailure();if(!e)break;let t=e.generationId.toString(),i=e.candidateId.toString(),n=ra(t,i),o=this.#r.get(n);if(this.#r.delete(n),this.#d===n&&(this.#l=null,this.#d=null),e.fatal)throw new Error(`public commit surface failed for generation ${t}, candidate ${i}: ${e.message}`);let s=this.#i.get(t);!s||!o||s.commitFailures.push(Object.freeze({candidateId:i,message:e.message}))}}#h(){if(this.#d!==null||this.#m())return null;for(;;){let e=this.#s.shift();if(!e)return null;let t=this.#r.get(e);if(!(!t||!this.#i.has(t.generationId))){try{this.#n.beginCommit(t.candidate)}catch(i){return{key:e,message:i instanceof Error?i.message:String(i)}}return this.#l=t.generationId,this.#d=e,null}}}#m(){for(let e of this.#i.values())if(e.committedReceipts.length>0||e.commitFailures.length>0)return!0;return!1}#g(e){let t=[];for(let i of this.#s){let n=this.#r.get(i),o=n?this.#i.get(n.generationId):void 0;!n||!o||(o.ordinal<e?(this.#n.discard(n.candidate),this.#r.delete(i)):t.push(i))}this.#s=t}};function W_(r,e){let t=new Cn(e.fontEnvironment);return Object.freeze({resources:t,device:new Nn(r,t,e.rendererFactory,e.privateCanvasFactory)})}function fP(r){return new Jc(r)}function H_(r,e){return Object.freeze({wire:r.target,configuration:r.configuration,resourceBindings:Object.freeze([]),surface:r.surface,viewports:Object.freeze([...r.viewports.values()]),extent:e.extent,interactionRevision:e.interactionRevision})}function j_(r,e){return Object.freeze({generationId:e.generationId,retainedRevisionKey:r,extent:e.extent,backdrop:e.backdrop,viewports:Object.freeze(e.viewports.map(t=>Object.freeze({key:t.key,treatment:t.treatment,batches:t.batches,decorations:t.decorations})))})}function Gm(r,e,t){return Wm(r,e,t.resources,t.configuration,t.viewports,"$.target")}function X_(r,e,t,i){let n=[...t.resources.values()],o=new Set(n.map(s=>oa(s)));for(let s of i.resourceBindings){let a=oa(s);if(o.has(a))return sr("duplicate_identity","$.resourceBindings",`duplicate resource binding ${a}`);o.add(a),n.push(s)}return Wm(r,e,n,i.configuration,i.viewports,"$")}function Wm(r,e,t,i,n,o){let s=new Set;for(let d of t){let u=oa(d);if(s.has(u))return sr("duplicate_identity",`${o}.resources`,`duplicate resource binding ${u}`);if(s.add(u),!(d.kind==="font"?r.hasFontResource(Wr(e,d.id,d.revision)):r.hasImageBinding(e,d.id,d.revision)))return sr("unknown_resource",`${o}.resources`,`resource ${u} was not installed for generation ${e}`)}let c=i.theme.font,l=Kc(s,c,`${o}.configuration.theme.font`,"font");if(l)return l;if(!r.hasFontResource(Wr(e,c.id,c.revision)))return sr("unknown_resource",`${o}.configuration.theme.font`,"theme font bytes are not installed");for(let d=0;d<n.length;d+=1){let h=n[d].batches;for(let p=0;p<h.length;p+=1){let x=h[p].content,g=`${o}.viewports[${d}].batches[${p}].content`;if(x.kind==="text"){let m=x.font,f=Kc(s,m,`${g}.font`,"font");if(f)return f;if(!r.hasFontResource(Wr(e,m.id,m.revision)))return sr("unknown_resource",`${g}.font`,"text font bytes are not installed")}else if(x.kind==="raster_image"){let m=Object.freeze({kind:"image",id:x.asset,revision:x.revision}),f=Kc(s,m,g);if(f)return f;let S=xo(e,x.asset,x.revision,x.frame);if(!r.hasImageFrameResource(S))return sr("unknown_resource",`${g}.frame`,"image frame RGBA8 payload is not installed")}}}return null}function Kc(r,e,t,i){let n=oa(e,i);return r.has(n)?null:sr("unknown_reference",t,`resource binding ${n} is not declared by the scene`)}function oa(r,e){return JSON.stringify([r.kind??e,r.id,r.revision])}function q_(r){return Object.freeze({status:"committed",candidateId:r.candidateId.toString(),layoutId:r.layoutId.toString(),extent:r.extent,interactionRevision:r.interactionRevision.toString(),hitMap:Object.freeze({regions:Object.freeze(r.hitMap.regions.map(e=>Object.freeze({control:e.logical,rect:e.rect})))})})}function ra(r,e){return JSON.stringify([r,e])}function zn(){return Object.freeze({status:"accepted"})}function Ye(r,e,t,i={}){return Object.freeze({status:"rejected",rejection:sr(r,e,t,i)})}function sr(r,e,t,i={}){return Object.freeze({code:r,path:e,message:t,...i})}function $_(r,e){return Object.freeze({...r,requestId:e})}function wr(r){return r instanceof Ze?Ye(r.code,r.path,r.message):Ye("invalid_schema","$",r instanceof Error?r.message:String(r))}function Zc(r){let e={};return r.generationId!==void 0&&(e.generationId=r.generationId.toString()),r.requestId!==void 0&&(e.requestId=r.requestId.toString()),Y_(r.transactionId)&&(e.transactionId=r.transactionId),Ye(na(r.code),r.path,r.message,e)}function Y_(r){return r!==null&&typeof r=="object"&&typeof r.session=="string"&&typeof r.stateCommit=="string"}function na(r){switch(r){case"conflicting_transaction":case"duplicate_identity":case"device_failure":case"incomplete_diff":case"invalid_schema":case"non_canonical_u64":case"not_installed":case"revision_gap":case"target_diff_disagreement":case"unknown_reference":case"unknown_resource":case"unknown_revision":case"wrong_base_revision":case"wrong_generation":return r;default:return"invalid_schema"}}export{Jc as PuzzleBrowserRenderer,G_ as createPuzzleAuthoringRenderer,fP as createPuzzleBrowserRenderer};
