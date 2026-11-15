version=ps_2_0
ctab.flags=33024
ctab.constant=AddColour,c1,1,6,type0
ctab.constant=MaskSampler,s1,1,6,type1
ctab.constant=MulColour,c0,1,2,type0
ctab.constant=SpriteSampler,s0,1,2,type2
ctab.string=AddColour,"AddColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=MaskSampler,"MaskSampler"
ctab.type=type1,object,sampler2d,1,1,1
ctab.string=MulColour,"MulColour"
ctab.string=SpriteSampler,"SpriteSampler"
ctab.type=type2,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, a0.xyzw
dcl sampler2d, s0.xyzw
dcl sampler2d, s1.xyzw
texld r0.xyzw, a0.xyzw, s0.xyzw
mov r1.x, a0.zzzz
mov r1.y, a0.wwww
texld r1.xyzw, r1.xyzw, s1.xyzw
mov r2.xyzw, c0.xyzw
mad r0.xyzw, r0.xyzw, r2.xyzw, c1.xyzw
mul r0.w, r1.wwww, r0.wwww
mov oC0.xyzw, r0.xyzw
