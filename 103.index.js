"use strict";(self.webpackChunk_escalier_lang_escalier=self.webpackChunk_escalier_lang_escalier||[]).push([[103],{103:(n,e,t)=>{t.a(n,(async(n,_)=>{try{t.r(e),t.d(e,{__wbg_log_358a2812406d1344:()=>o.CR,__wbg_new_b51585de1b234aff:()=>o.Tc,__wbg_set_841ac57cff3d672b:()=>o.n0,__wbg_set_wasm:()=>o.oT,__wbindgen_object_clone_ref:()=>o.m_,__wbindgen_object_drop_ref:()=>o.ug,__wbindgen_string_new:()=>o.h4,__wbindgen_throw:()=>o.Or,compile:()=>o.MY,parse:()=>o.Qc});var r=t(960),o=t(785),c=n([r]);r=(c.then?(await c)():c)[0],(0,o.oT)(r),_()}catch(n){_(n)}}))},785:(n,e,t)=>{let _;function r(n){_=n}t.d(e,{CR:()=>k,MY:()=>m,Or:()=>A,Qc:()=>T,Tc:()=>O,h4:()=>j,m_:()=>x,n0:()=>C,oT:()=>r,ug:()=>v}),n=t.hmd(n);const o=new Array(128).fill(void 0);function c(n){return o[n]}o.push(void 0,null,!0,!1);let i=o.length;function l(n){const e=c(n);return function(n){n<132||(o[n]=i,i=n)}(n),e}function d(n){i===o.length&&o.push(o.length+1);const e=i;return i=o[e],o[e]=n,e}let u=new("undefined"==typeof TextDecoder?(0,n.require)("util").TextDecoder:TextDecoder)("utf-8",{ignoreBOM:!0,fatal:!0});u.decode();let f=null;function a(){return null!==f&&0!==f.byteLength||(f=new Uint8Array(_.memory.buffer)),f}function g(n,e){return n>>>=0,u.decode(a().subarray(n,n+e))}let b=0,w=new("undefined"==typeof TextEncoder?(0,n.require)("util").TextEncoder:TextEncoder)("utf-8");const s="function"==typeof w.encodeInto?function(n,e){return w.encodeInto(n,e)}:function(n,e){const t=w.encode(n);return e.set(t),{read:n.length,written:t.length}};function h(n,e,t){if(void 0===t){const t=w.encode(n),_=e(t.length,1)>>>0;return a().subarray(_,_+t.length).set(t),b=t.length,_}let _=n.length,r=e(_,1)>>>0;const o=a();let c=0;for(;c<_;c++){const e=n.charCodeAt(c);if(e>127)break;o[r+c]=e}if(c!==_){0!==c&&(n=n.slice(c)),r=t(r,_,_=c+3*n.length,1)>>>0;const e=a().subarray(r+c,r+_);c+=s(n,e).written}return b=c,r}let p=null;function y(){return null!==p&&0!==p.byteLength||(p=new Int32Array(_.memory.buffer)),p}function m(n,e){try{const o=_.__wbindgen_add_to_stack_pointer(-16),c=h(n,_.__wbindgen_malloc,_.__wbindgen_realloc),i=b,d=h(e,_.__wbindgen_malloc,_.__wbindgen_realloc),u=b;_.compile(o,c,i,d,u);var t=y()[o/4+0],r=y()[o/4+1];if(y()[o/4+2])throw l(r);return l(t)}finally{_.__wbindgen_add_to_stack_pointer(16)}}function T(n){try{const r=_.__wbindgen_add_to_stack_pointer(-16),o=h(n,_.__wbindgen_malloc,_.__wbindgen_realloc),c=b;_.parse(r,o,c);var e=y()[r/4+0],t=y()[r/4+1];if(y()[r/4+2])throw l(t);return l(e)}finally{_.__wbindgen_add_to_stack_pointer(16)}}function k(n,e){console.log(g(n,e))}function v(n){l(n)}function x(n){return d(c(n))}function j(n,e){return d(g(n,e))}function C(n,e,t){c(n)[l(e)]=l(t)}function O(){return d(new Object)}function A(n,e){throw new Error(g(n,e))}},960:(n,e,t)=>{var _=t(785);n.exports=t.v(e,n.id,"28cb3d66fd8de0470255",{"./index_bg.js":{__wbg_log_358a2812406d1344:_.CR,__wbindgen_object_drop_ref:_.ug,__wbindgen_object_clone_ref:_.m_,__wbindgen_string_new:_.h4,__wbg_set_841ac57cff3d672b:_.n0,__wbg_new_b51585de1b234aff:_.Tc,__wbindgen_throw:_.Or}})}}]);