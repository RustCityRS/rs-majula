version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseSampler,s0,1,2,type0
ctab.constant=DistanceFogColour,c0,1,2,type1
ctab.constant=HeightFogColour,c2,1,10,type1
ctab.constant=HeightFogPlane,c1,1,6,type2
ctab.constant=SpecularExponent,c3,1,14,type2
ctab.string=DiffuseSampler,"DiffuseSampler"
ctab.padding=171,1
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=DistanceFogColour,"DistanceFogColour"
ctab.padding=171,2
ctab.type=type1,vector,float,1,3,1
ctab.string=HeightFogColour,"HeightFogColour"
ctab.string=HeightFogPlane,"HeightFogPlane"
ctab.padding=171,1
ctab.type=type2,vector,float,1,4,1
ctab.string=SpecularExponent,"SpecularExponent"
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,3
def c4.xyzw, -1, 1, 0, 0
dcl position0, a0.xyzw
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
pow r2.w, r1.xxxx, c3.xxxx
mul r1.x, r2.wwww, a2.wwww
mad r1.yzw, r0.wzyx, v0.wzyx, c4.xxxx
mov r2.y, c4.yyyy
mad r1.yzw, c3.yyyy, r1.xyzw, r2.yyyy
mul r1.xyz, r1.wzyx, r1.xxxx
mad r1.xyz, r1.xyzw, r0.wwww, v1.xyzw
dp3 r0.w, a3.xyzw, a3.xyzw
rsq r0.w, r0.wwww
rcp r0.w, r0.wwww
cmp r1.xyz, -r0.wwww, v1.xyzw, r1.xyzw
mul r0.w_sat, a0.zzzz, c1.yyyy
lrp r2.xyz, r0.wwww, c2.xyzw, r1.xyzw
add r0.w, -r0.wwww, c4.yyyy
mul r1.xyz, r0.wwww, v0.xyzw
lrp r3.xyz, a0.wwww, c0.xyzw, r2.xyzw
add r0.w, -a0.wwww, c4.yyyy
mul r1.xyz, r0.wwww, r1.xyzw
mad r0.xyz, r0.xyzw, r1.xyzw, r3.xyzw
mov r0.w, v0.wwww
mov oC0.xyzw, r0.xyzw
