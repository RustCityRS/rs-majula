version=vs_2_0
ctab.flags=33024
ctab.constant=PosAndTexCoords,c0,3,2,type0
ctab.string=PosAndTexCoords,"PosAndTexCoords"
ctab.type=type0,vector,float,1,4,3
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c3.xyzw, 1, 0, 0, 0
dcl texcoord0, v0.xyzw
mova a0.x, v0.xxxx
mov r0.xy, c3.xyzw
mad oPos.xyzw, c0.xyxx[a0.xxxx], r0.xxyy, r0.yyyx
mov oT0.xy, c0.zwzw[a0.xxxx]
