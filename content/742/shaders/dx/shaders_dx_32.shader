version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseSampler,s0,1,2,type0
ctab.constant=EnvironmentSampler,s1,1,6,type1
ctab.string=DiffuseSampler,"DiffuseSampler"
ctab.padding=171,1
ctab.type=type0,object,sampler2d,1,1,1
ctab.string=EnvironmentSampler,"EnvironmentSampler"
ctab.padding=171,1
ctab.type=type1,object,samplercube,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c0.xyzw, 1, 0, 0, 0
dcl position0, a0.xy
dcl position0, v0.xyzw
dcl position0, v1.xyzw
dcl position0, a1.xyz
dcl sampler2d, s0.xyzw
dcl sampler3d, s1.xyzw
texld r0.xyzw, a1.xyzw, s1.xyzw
texld r1.xyzw, a0.xyzw, s0.xyzw
mul r0.xyz, r0.xyzw, r1.wwww
mad r0.xyz, r0.xyzw, v1.wwww, v1.xyzw
add r0.w, -r1.wwww, c0.xxxx
mul r2.xyz, r0.wwww, v0.xyzw
mad r0.xyz, r1.xyzw, r2.xyzw, r0.xyzw
mov r0.w, v0.wwww
mov oC0.xyzw, r0.xyzw
