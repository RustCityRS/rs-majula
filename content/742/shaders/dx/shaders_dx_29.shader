version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseSampler,s0,1,2,type0
ctab.string=DiffuseSampler,"DiffuseSampler"
ctab.padding=171,1
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c0.xyzw, 0, 0, 0, 0
dcl position0, a0.xy
dcl position0, v0.xyzw
dcl position0, v1.xyz
dcl sampler2d, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mov r1.xyz, v1.xyzw
mov r1.w, c0.xxxx
mad r0.xyzw, r0.xyzw, v0.xyzw, r1.xyzw
mov oC0.xyzw, r0.xyzw
