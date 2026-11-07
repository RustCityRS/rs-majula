version=vs_1_1
ctab.flags=295168
ctab.constant=fogPlanes,c10,1,42,type0
ctab.constant=modelViewMatrix,c0,4,2,type1
ctab.constant=projectionMatrix,c4,4,18,type1
ctab.constant=textureMatrix,c8,2,34,type2
ctab.constant=time,c11,1,46,type3
ctab.string=fogPlanes,"fogPlanes"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.type=type1,matrix_columns,float,4,4,1
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,1
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=time,"time"
ctab.padding=171,3
ctab.type=type3,scalar,float,1,1,1
ctab.string=target,"vs_1_1"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c12.xyzw, 1, 0, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dp4 r0.x, v0.xyzw, c0.xyzw
dp4 r0.y, v0.xyzw, c1.xyzw
dp4 r0.w, v0.xyzw, c3.xyzw
dp4 r0.z, v0.xyzw, c2.xyzw
dp4 oPos.x, r0.xyzw, c4.xyzw
dp4 oPos.y, r0.xyzw, c5.xyzw
dp4 oPos.z, r0.xyzw, c6.xyzw
dp4 oPos.w, r0.xyzw, c7.xyzw
add r0.xyzw, -r0.zzzz, -c10.xxxz
mul oD0.xyzw, r0.xyzw, c10.yyyw
mad r0.xyz, v1.xyxw, c12.xxyw, c12.yyxw
dp3 oT0.x, r0.xyzw, c8.xyww
dp3 oT0.y, r0.xyzw, c9.xyww
mov oT0.z, c11.xxxx
