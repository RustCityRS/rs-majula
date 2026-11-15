version=ps_2_0
ctab.flags=33024
ctab.constant=causticSampler3D,s0,1,2,type0
ctab.constant=fogColour,c0,1,2,type1
ctab.string=causticSampler3D,"causticSampler3D"
ctab.padding=171,3
ctab.type=type0,object,sampler3d,1,1,1
ctab.string=fogColour,"fogColour"
ctab.padding=171,2
ctab.type=type1,vector,float,1,4,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, a0.xyz
dcl position0, v0.x
dcl samplercube, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
lrp r1.xyzw, v0.xxxx, c0.xyzw, r0.xyzw
mov oC0.xyzw, r1.xyzw
