version=ps_3_0
ctab.flags=33024
ctab.constant=pixelOffsetAndBloomScale,c0,1,2,type0
ctab.constant=sampleSize,c1,1,6,type1
ctab.constant=sceneTex,s0,1,2,type2
ctab.string=pixelOffsetAndBloomScale,"pixelOffsetAndBloomScale"
ctab.padding=171,3
ctab.type=type0,vector,float,1,4,1
ctab.string=sampleSize,"sampleSize"
ctab.padding=171,1
ctab.type=type1,vector,float,1,2,1
ctab.string=sceneTex,"sceneTex"
ctab.padding=171,3
ctab.type=type2,object,sampler2d,1,1,1
ctab.string=target,"ps_3_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c2.xyzw, 0.088584304, 0.091396265, -2, -3
def c3.xyzw, 1, 0, 0, 0
def c4.xyzw, 0.08065692, 0.068989515, 3, -4
def c5.xyzw, 0.055434637, 4, -5, 0.04184426
def c6.xyzw, 5, -6, 0.029672023, 7
def c7.xyzw, 0.019765828, 7, -8, 0.012369139
dcl texcoord0, v0.xy
dcl sampler2d, s0.xyzw
add r0.xy, -c1.xyzw, v0.xyzw
add r0.xy, r0.xyzw, -c0.xyzw
texld r0.xyzw, r0.xyzw, s0.xyzw
mul r0.xyz, r0.xyzw, c2.xxxx
add r1.xy, -c0.xyzw, v0.xyzw
texld r1.xyzw, r1.xyzw, s0.xyzw
mad r0.xyz, r1.xyzw, c2.yyyy, r0.xyzw
add r1.xy, c1.xyzw, v0.xyzw
add r1.xy, r1.xyzw, -c0.xyzw
texld r1.xyzw, r1.xyzw, s0.xyzw
mad r0.xyz, r1.xyzw, c2.xxxx, r0.xyzw
mov r1.xy, c1.xyzw
mad r2.xyzw, r1.xyxy, c2.zzww, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c4.xxxx, r0.xyzw
mad r1.zw, r1.xyxy, -c2.zzzz, v0.xyxy
add r1.zw, r1.xyzw, -c0.xyxy
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c4.xxxx, r0.xyzw
mad r0.xyz, r2.xyzw, c4.yyyy, r0.xyzw
mad r2.xyzw, r1.xyxy, c4.zzww, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c4.yyyy, r0.xyzw
mad r0.xyz, r2.xyzw, c5.xxxx, r0.xyzw
mad r2.xyzw, r1.xyxy, c5.yyzz, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c5.xxxx, r0.xyzw
mad r0.xyz, r2.xyzw, c5.wwww, r0.xyzw
mad r2.xyzw, r1.xyxy, c6.xxyy, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c5.wwww, r0.xyzw
mad r0.xyz, r2.xyzw, c6.zzzz, r0.xyzw
mad r2.xyzw, r1.xyxy, -c6.yyww, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c6.zzzz, r0.xyzw
mad r0.xyz, r2.xyzw, c7.xxxx, r0.xyzw
mad r2.xyzw, r1.xyxy, c7.yyzz, v0.xyxy
add r1.zw, r2.xyxy, -c0.xyxy
add r2.xy, r2.zwzw, -c0.xyzw
texld r2.xyzw, r2.xyzw, s0.xyzw
texld r3.xyzw, r1.zwzw, s0.xyzw
mad r0.xyz, r3.xyzw, c7.xxxx, r0.xyzw
mad r0.xyz, r2.xyzw, c7.wwww, r0.xyzw
mad r1.xy, r1.xyzw, -c7.zzzz, v0.xyzw
add r1.xy, r1.xyzw, -c0.xyzw
texld r1.xyzw, r1.xyzw, s0.xyzw
mad oC0.xyz, r1.xyzw, c7.wwww, r0.xyzw
mov oC0.w, c3.xxxx
