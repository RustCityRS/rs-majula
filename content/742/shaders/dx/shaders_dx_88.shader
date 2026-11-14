version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseSampler,s0,1,2,type0
ctab.constant=SpecularExponent,c0,1,2,type1
ctab.string=DiffuseSampler,"DiffuseSampler"
ctab.padding=171,1
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=SpecularExponent,"SpecularExponent"
ctab.padding=171,3
ctab.type=type1,vector,float,1,4,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c1.xyzw, -1, 1, 0, 0
dcl position0, a0.xy
dcl position0, v0.xyzw
dcl position0, v1.xyz
dcl position0, a1.xyz
dcl position0, a2.xyzw
dcl position0, a3.xyz
dcl sampler2d, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mov r1.xyz, a3.xyzw
dp3 r1.w, a1.xyzw, r1.xyzw
add r1.w, r1.wwww, r1.wwww
mad r1.xyz, r1.xyzw, -r1.wwww, a1.xyzw
dp3 r1.x_sat, r1.xyzw, a2.xyzw
pow r2.w, r1.xxxx, c0.xxxx
mul r1.x, r2.wwww, a2.wwww
mad r1.yzw, r0.wzyx, v0.wzyx, c1.xxxx
mov r2.y, c1.yyyy
mad r1.yzw, c0.yyyy, r1.xyzw, r2.yyyy
mul r1.xyz, r1.wzyx, r1.xxxx
mad r1.xyz, r1.xyzw, r0.wwww, v1.xyzw
dp3 r0.w, a3.xyzw, a3.xyzw
rsq r0.w, r0.wwww
rcp r0.w, r0.wwww
cmp r1.xyz, -r0.wwww, v1.xyzw, r1.xyzw
mad r0.xyz, r0.xyzw, v0.xyzw, r1.xyzw
mov r0.w, v0.wwww
mov oC0.xyzw, r0.xyzw
