version=ps_2_0
ctab.flags=33024
ctab.constant=pixelOffsetAndBloomScale,c0,1,2,type0
ctab.constant=sceneTex,s0,1,2,type1
ctab.string=pixelOffsetAndBloomScale,"pixelOffsetAndBloomScale"
ctab.padding=171,3
ctab.type=type0,vector,float,1,4,1
ctab.string=sceneTex,"sceneTex"
ctab.padding=171,3
ctab.type=type1,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c1.xyzw, -0.9999, 1, 0, 1
dcl position0, a0.xy
dcl sampler2d, s0.xyzw
add r0.xy, a0.xyzw, -c0.xyzw
texld r0.xyzw, r0.xyzw, s0.xyzw
add r0.w, r0.wwww, c1.xxxx
cmp r0.xyz, r0.wwww, r0.xyzw, c1.wzyx
mov r0.w, c1.wwww
mov oC0.xyzw, r0.xyzw
