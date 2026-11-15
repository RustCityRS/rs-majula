version=ps_2_0
ctab.flags=33024
ctab.constant=DistanceFogColour,c0,1,2,type0
ctab.constant=EnvMapSampler,s1,1,6,type1
ctab.constant=NormalSampler,s0,1,2,type2
ctab.constant=SunColourAndWaveExponent,c2,1,10,type3
ctab.constant=SunDirectionAndExponent,c1,1,6,type3
ctab.string=DistanceFogColour,"DistanceFogColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=EnvMapSampler,"EnvMapSampler"
ctab.padding=171,2
ctab.type=type1,object,samplercube,1,1,1
ctab.string=NormalSampler,"NormalSampler"
ctab.padding=171,2
ctab.type=type2,object,sampler3d,1,1,1
ctab.string=SunColourAndWaveExponent,"SunColourAndWaveExponent"
ctab.padding=171,3
ctab.type=type3,vector,float,1,4,1
ctab.string=SunDirectionAndExponent,"SunDirectionAndExponent"
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c3.xyzw, 2, -1, 1, 0
dcl position0, a0.xyz
dcl position0, v0.xyzw
dcl position0, v1.xyzw
dcl position0, a1.xyz
dcl samplercube, s0.xyzw
dcl sampler3d, s1.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
nrm r1.xyz, a1.xyzw
mad r0.xyz, r0.xyzw, c3.xxxx, c3.yyyy
dp3 r0.w, r1.xyzw, r0.xyzw
add r0.w, r0.wwww, r0.wwww
mad r1.xyz, r0.xyzw, -r0.wwww, r1.xyzw
dp3 r0.w, r1.xyzw, r0.xyzw
dp3 r1.w, c1.xyzw, r0.xyzw
add r1.w, r1.wwww, r1.wwww
abs r0.x, r0.wwww
add r0.x, -r0.xxxx, c3.zzzz
add r0.y, -r0.xxxx, c3.zzzz
dp3 r0.z_sat, c1.xyzw, r1.xyzw
texld r2.xyzw, r1.xyzw, s1.xyzw
pow r2.w, r0.zzzz, c1.wwww
mad r2.w, r1.wwww, r0.yyyy, r2.wwww
mad r0.yzw, r2.wwww, c2.wzyx, r2.wzyx
mad r0.yzw, c0.wzyx, r0.xyzw, -c0.wzyx
mad r0.xyz, r0.xxxx, r0.wzyx, c0.xyzw
mov r0.w, c3.zzzz
mov r1.xyzw, v0.xyzw
mad r0.xyzw, r0.xyzw, r1.xyzw, v1.xyzw
mov oC0.xyzw, r0.xyzw
