version=ps_2_0
ctab.flags=33024
ctab.constant=SpriteSampler,s0,1,2,type0
ctab.string=SpriteSampler,"SpriteSampler"
ctab.padding=171,2
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, a0.xy
dcl position0, v0.xyzw
dcl sampler2d, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mul r0.xyzw, r0.xyzw, v0.xyzw
mov oC0.xyzw, r0.xyzw
