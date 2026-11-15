version=ps_2_0
ctab.flags=33024
ctab.constant=causticSampler3D,s1,1,6,type0
ctab.constant=diffuseTexture,s0,1,2,type1
ctab.constant=waterFogColour,c0,1,2,type2
ctab.constant=waterParams,c1,1,6,type3
ctab.string=causticSampler3D,"causticSampler3D"
ctab.padding=171,3
ctab.type=type0,object,sampler3d,1,1,1
ctab.string=diffuseTexture,"diffuseTexture"
ctab.padding=171,1
ctab.type=type1,object,sampler2d,1,1,1
ctab.string=waterFogColour,"waterFogColour"
ctab.padding=171,1
ctab.type=type2,vector,float,1,4,1
ctab.string=waterParams,"waterParams"
ctab.type=type3,vector,float,1,2,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c2.xyzw, 0.0025, 0, 0, 0
dcl position0, v0.xyzw
dcl position0, a0.xy
dcl position0, a1.x
dcl position0, a2.xyzw
dcl sampler2d, s0.xyzw
dcl samplercube, s1.xyzw
texld r0.xyzw, a2.xyzw, s1.xyzw
texld r1.xyzw, a0.xyzw, s0.xyzw
mul r0.x, r0.wwww, c1.xxxx
mul r0.y_sat, a2.wwww, c2.xxxx
mul r1.xyzw, r1.xyzw, v0.xyzw
mad r0.xyz, r0.xxxx, r0.yyyy, r1.xyzw
mov r0.w_sat, a1.xxxx
lrp r1.xyz, r0.wwww, c0.xyzw, r0.xyzw
mov oC0.xyzw, r1.xyzw
