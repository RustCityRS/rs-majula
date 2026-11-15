version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseColour,c0,1,2,type0
ctab.constant=DiffuseSampler,s0,1,2,type1
ctab.string=DiffuseColour,"DiffuseColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=DiffuseSampler,"DiffuseSampler"
ctab.padding=171,1
ctab.type=type1,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, v0.xyzw
dcl position0, a0.xy
dcl sampler2d, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mul r0.xyzw, r0.xyzw, c0.xyzw
mul r0.xyzw, r0.xyzw, v0.xyzw
mov oC0.xyzw, r0.xyzw
