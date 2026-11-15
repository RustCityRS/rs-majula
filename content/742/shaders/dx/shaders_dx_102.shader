version=ps_2_0
ctab.flags=33024
ctab.constant=DiffuseSampler,s0,1,2,type0
ctab.constant=DistanceFogColour,c0,1,2,type1
ctab.constant=HeightFogColour,c2,1,10,type1
ctab.constant=HeightFogPlane,c1,1,6,type2
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
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c3.xyzw, 1, 0, 0, 0
dcl position0, a0.xyzw
dcl position0, v0.xyzw
dcl position0, v1.xyz
dcl sampler2d, s0.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mul r1.w_sat, a0.zzzz, c1.yyyy
add r1.x, -r1.wwww, c3.xxxx
lrp r2.xyz, r1.wwww, c2.xyzw, v1.xyzw
lrp r3.xyz, a0.wwww, c0.xyzw, r2.xyzw
mul r1.xyz, r1.xxxx, v0.xyzw
add r1.w, -a0.wwww, c3.xxxx
mul r1.xyz, r1.wwww, r1.xyzw
mov r1.w, v0.wwww
mov r3.w, c3.yyyy
mad r0.xyzw, r0.xyzw, r1.xyzw, r3.xyzw
mov oC0.xyzw, r0.xyzw
