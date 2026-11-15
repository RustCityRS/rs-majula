version=ps_2_0
ctab.flags=33024
ctab.constant=EnvMapSampler,s1,1,6,type0
ctab.constant=NormalSampler,s0,1,2,type1
ctab.constant=SunColourAndWaveExponent,c1,1,6,type2
ctab.constant=SunDirectionAndExponent,c0,1,2,type2
ctab.constant=WaveIntensityAndBreakWaterDepthAndOffset,c2,1,10,type2
ctab.string=EnvMapSampler,"EnvMapSampler"
ctab.padding=171,2
ctab.type=type0,object,samplercube,1,1,1
ctab.string=NormalSampler,"NormalSampler"
ctab.padding=171,2
ctab.type=type1,object,sampler3d,1,1,1
ctab.string=SunColourAndWaveExponent,"SunColourAndWaveExponent"
ctab.padding=171,3
ctab.type=type2,vector,float,1,4,1
ctab.string=SunDirectionAndExponent,"SunDirectionAndExponent"
ctab.string=WaveIntensityAndBreakWaterDepthAndOffset,"WaveIntensityAndBreakWaterDepthAndOffset"
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,3
def c3.xyzw, -4, 1, 0.025, 0
def c4.xyzw, 2, -1, 1.0001, -0.5
def c5.xyzw, -2, 3, 0, 0
dcl position0, a0.xyzw
dcl position0, v0.xyzw
dcl position0, v1.xyzw
dcl position0, a1.xyz
dcl samplercube, s0.xyzw
dcl sampler3d, s1.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mul r1.w_sat, a0.wwww, c3.zzzz
mad r1.x, r1.wwww, c5.xxxx, c5.yyyy
mul r1.y, r1.wwww, r1.wwww
mul r1.x, r1.yyyy, r1.xxxx
nrm r2.xyz, a1.xyzw
mad r0.xyz, r0.xyzw, c4.xxxx, c4.yyyy
dp3 r2.w, r2.xyzw, r0.xyzw
add r2.w, r2.wwww, r2.wwww
mad r2.xyz, r0.xyzw, -r2.wwww, r2.xyzw
dp3 r2.w, r2.xyzw, r0.xyzw
abs r2.w, r2.wwww
add r2.w, -r2.wwww, -c4.yyyy
mul r0.x, r0.wwww, c2.wwww
mad r0.y, c2.xxxx, r0.wwww, c2.yyyy
rcp r0.z, c2.zzzz
mad r0.x_sat, a0.wwww, r0.zzzz, -r0.xxxx
mul r2.w, r0.xxxx, r2.wwww
mul r1.w, r1.xxxx, r2.wwww
add r2.w, -r0.xxxx, c4.zzzz
pow r0.z, r2.wwww, c1.wwww
add r2.w, r0.zzzz, c4.wwww
mul r2.w, r2.wwww, r2.wwww
mad r2.w, r2.wwww, c3.xxxx, c3.yyyy
texld r3.xyzw, r2.xyzw, s1.xyzw
dp3 r3.w_sat, c0.xyzw, r2.xyzw
pow r0.z, r3.wwww, c0.wwww
mul r2.xyz, r0.zzzz, c1.xyzw
mul r4.xyz, r0.xxxx, r2.xyzw
mov r1.xyz, r3.xyzw
lrp r3.xyzw, r2.wwww, r0.yyyy, r1.xyzw
mov r4.w, c3.wwww
add r0.xyzw, r3.xyzw, r4.xyzw
mov r1.xyzw, v0.xyzw
mad r0.xyzw, r0.xyzw, r1.xyzw, v1.xyzw
mov oC0.xyzw, r0.xyzw
