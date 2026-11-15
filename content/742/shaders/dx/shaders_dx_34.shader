version=ps_2_0
ctab.flags=33024
ctab.constant=bloomTex1,s1,1,6,type0
ctab.constant=params,c0,1,2,type1
ctab.constant=pixelOffsetAndBloomScale,c1,1,6,type1
ctab.constant=sceneTex,s0,1,2,type2
ctab.string=bloomTex1,"bloomTex1"
ctab.padding=171,2
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=params,"params"
ctab.padding=171,1
ctab.type=type1,vector,float,1,4,1
ctab.string=pixelOffsetAndBloomScale,"pixelOffsetAndBloomScale"
ctab.string=sceneTex,"sceneTex"
ctab.padding=171,2
ctab.type=type2,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c2.xyzw, 0.2126, 0.7152, 0.0722, 1
def c3.xyzw, 0.99, 0.01, 1.01, 0
dcl position0, a0.xy
dcl sampler2d, s0.xyzw
dcl sampler2d, s1.xyzw
rcp r0.w, c0.zzzz
add r0.xy, a0.xyzw, -c1.xyzw
mad r1.x, a0.xxxx, c1.zzzz, -c1.xxxx
mad r1.y, a0.yyyy, c1.wwww, -c1.yyyy
texld r2.xyzw, r0.xyzw, s0.xyzw
texld r1.xyzw, r1.xyzw, s1.xyzw
dp3 r0.x, c2.xyzw, r2.xyzw
mad r0.y, r0.xxxx, c3.xxxx, c3.yyyy
mad r0.x, r0.xxxx, c3.xxxx, c3.zzzz
rcp r0.x, r0.xxxx
mad r0.z, r0.yyyy, r0.wwww, c2.wwww
mul r0.z, r0.zzzz, r0.yyyy
rcp r0.y, r0.yyyy
mul r0.x, r0.xxxx, r0.zzzz
mul r0.x, r0.yyyy, r0.xxxx
mul r1.xyzw, r1.xyzw, c0.yyyy
mov r2.w, c2.wwww
mad r0.xyzw, r2.xyzw, r0.xxxx, r1.xyzw
mov oC0.xyzw, r0.xyzw
