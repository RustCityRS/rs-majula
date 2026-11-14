version=vs_2_0
ctab.flags=33024
ctab.constant=fogPlanes,c6,1,26,type0
ctab.constant=modelViewProjectionMatrix,c0,4,2,type1
ctab.constant=textureMatrix,c4,2,18,type2
ctab.constant=time,c7,1,30,type3
ctab.string=fogPlanes,"fogPlanes"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=modelViewProjectionMatrix,"modelViewProjectionMatrix"
ctab.padding=171,2
ctab.type=type1,matrix_columns,float,4,4,1
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,2
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=time,"time"
ctab.padding=171,3
ctab.type=type3,scalar,float,1,1,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c8.xyzw, 1, 0, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
mad r0.xyz, v1.xyxw, c8.xxyw, c8.yyxw
dp3 oT0.x, r0.xyzw, c4.xyww
dp3 oT0.y, r0.xyzw, c5.xyww
dp4 r0.x, v0.xyzw, c2.xyzw
add r0.y, r0.xxxx, c8.xxxx
mov oPos.z, r0.xxxx
mad r0.x, r0.yyyy, c6.yyyy, c6.xxxx
max r0.x, r0.xxxx, c8.yyyy
min r0.x, r0.xxxx, c8.xxxx
add oD0.x, -r0.xxxx, c8.xxxx
mov oT0.z, c7.xxxx
