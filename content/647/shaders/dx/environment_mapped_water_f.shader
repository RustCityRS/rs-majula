version=ps_2_0
ctab.flags=295168
ctab.constant=breakWaterDepth,c4,1,18,type0
ctab.constant=breakWaterOffset,c5,1,22,type0
ctab.constant=envMapSampler,s1,1,6,type1
ctab.constant=normalSampler,s0,1,2,type2
ctab.constant=sunColour,c2,1,10,type3
ctab.constant=sunDir,c1,1,6,type4
ctab.constant=sunExponent,c3,1,14,type0
ctab.string=breakWaterDepth,"breakWaterDepth"
ctab.type=type0,scalar,float,1,1,1
ctab.string=breakWaterOffset,"breakWaterOffset"
ctab.string=envMapSampler,"envMapSampler"
ctab.padding=171,1
ctab.type=type1,object,samplercube,1,1,1
ctab.string=normalSampler,"normalSampler"
ctab.padding=171,2
ctab.type=type2,object,sampler3d,1,1,1
ctab.string=sunColour,"sunColour"
ctab.padding=171,2
ctab.type=type3,vector,float,1,4,1
ctab.string=sunDir,"sunDir"
ctab.padding=171,1
ctab.type=type4,vector,float,1,3,1
ctab.string=sunExponent,"sunExponent"
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c0.xyzw, 2, -1, 1, 0
dcl position0, a0.xyz
dcl position0, a1.x
dcl position0, a2.xyz
dcl samplercube, s0.xyzw
dcl sampler3d, s1.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mad r0.xyz, r0.xyzw, c0.xxxx, c0.yyyy
mul r0.w, r0.wwww, c5.xxxx
nrm r1.xyz, a2.xyzw
dp3 r1.w, r1.xyzw, r0.xyzw
add r2.w, r1.wwww, r1.wwww
abs r1.w, r1.wwww
add r1.w, -r1.wwww, c0.zzzz
mul r1.w, r1.wwww, r1.wwww
mad r0.xyz, r0.xyzw, -r2.wwww, r1.xyzw
dp3 r1.x_sat, c1.xyzw, r0.xyzw
texld r2.xyzw, r0.xyzw, s1.xyzw
pow r0.x, r1.xxxx, c3.xxxx
mul r3.xyzw, r0.xxxx, c2.xyzw
rcp r0.x, c4.xxxx
mad r0.x_sat, a1.xxxx, r0.xxxx, -r0.wwww
mul r2.w, r0.xxxx, r1.wwww
mad r0.xyzw, r3.xyzw, r0.xxxx, r2.xyzw
mov oC0.xyzw, r0.xyzw
