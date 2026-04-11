' ============================================================================
' QBNex Standard Library - Simple Game Engine Example
' ============================================================================
' Demonstrates vector math, collections, and OOP for game development
' ============================================================================

'$INCLUDE:'../qbnex_stdlib.bas'

' ============================================================================
' Game Entity Types
' ============================================================================

TYPE GameObject
    __vtable_id AS LONG
    ID AS LONG
    NAME AS STRING * 32
    Position AS QBNex_Vec3
    Velocity AS QBNex_Vec3
    Active AS LONG
END TYPE

' ============================================================================
' Game State
' ============================================================================

TYPE GameState
    Entities AS QBNex_List
    EntityCount AS LONG
    DeltaTime AS DOUBLE
    TotalTime AS DOUBLE
END TYPE

' ============================================================================
' Class IDs
' ============================================================================
DIM SHARED GameObjectClassID AS LONG

' ============================================================================
' Initialize Game System
' ============================================================================
SUB InitGameSystem ()
    GameObjectClassID = QBNEX_RegisterClass("GameObject", 0)
    QBNEX_RegisterMethod GameObjectClassID, "Update", 1
    QBNEX_RegisterMethod GameObjectClassID, "Render", 2
    
    PRINT "Game system initialized"
    PRINT "  GameObject class registered (ID: "; GameObjectClassID; ")"
    PRINT
END SUB

' ============================================================================
' Entity Management
' ============================================================================

SUB CreateEntity (game AS GameState, NAME AS STRING, x AS DOUBLE, y AS DOUBLE, z AS DOUBLE)
    DIM entity AS STRING
    DIM POS AS QBNex_Vec3
    DIM vel AS QBNex_Vec3
    
    Vec3_Set POS, x, y, z
    Vec3_Set vel, 0, 0, 0
    
    ' Store as formatted string (simplified)
    entity = NAME + "|"
    entity = entity + LTRIM$(STR$(x)) + ","
    entity = entity + LTRIM$(STR$(y)) + ","
    entity = entity + LTRIM$(STR$(z))
    
    List_Add game.Entities, entity
    game.EntityCount = game.EntityCount + 1
END SUB

SUB ParseEntity (entityStr AS STRING, NAME AS STRING, POS AS QBNex_Vec3)
    DIM pipePos AS LONG
    DIM commaPos1 AS LONG
    DIM commaPos2 AS LONG
    DIM coords AS STRING
    
    pipePos = INSTR(entityStr, "|")
    NAME = LEFT$(entityStr, pipePos - 1)
    coords = MID$(entityStr, pipePos + 1)
    
    commaPos1 = INSTR(coords, ",")
    commaPos2 = INSTR(commaPos1 + 1, coords, ",")
    
    POS.X = VAL(LEFT$(coords, commaPos1 - 1))
    POS.Y = VAL(MID$(coords, commaPos1 + 1, commaPos2 - commaPos1 - 1))
    POS.Z = VAL(MID$(coords, commaPos2 + 1))
END SUB

' ============================================================================
' Physics System
' ============================================================================

SUB ApplyGravity (vel AS QBNex_Vec3, deltaTime AS DOUBLE)
    DIM gravity AS QBNex_Vec3
    DIM gravityForce AS QBNex_Vec3
    
    Vec3_Set gravity, 0, -9.8, 0
    Vec3_Scale gravityForce, gravity, deltaTime
    Vec3_Add vel, vel, gravityForce
END SUB

SUB UpdatePosition (POS AS QBNex_Vec3, vel AS QBNex_Vec3, deltaTime AS DOUBLE)
    DIM displacement AS QBNex_Vec3
    Vec3_Scale displacement, vel, deltaTime
    Vec3_Add POS, POS, displacement
END SUB

FUNCTION CheckCollision& (pos1 AS QBNex_Vec3, pos2 AS QBNex_Vec3, radius AS DOUBLE)
    DIM distance AS DOUBLE
    distance = Vec3_Distance(pos1, pos2)
    CheckCollision = (distance < radius)
END FUNCTION

' ============================================================================
' Main Program
' ============================================================================

CLS
PRINT "========================================================================"
PRINT "QBNex Standard Library - Simple Game Engine Example"
PRINT "========================================================================"
PRINT

' Initialize systems
InitGameSystem

DIM game AS GameState
List_Init game.Entities
game.EntityCount = 0
game.DeltaTime = 0.016 ' ~60 FPS
game.TotalTime = 0

PRINT "Press any key to start simulation..."
SLEEP
CLS

' ============================================================================
' Create Game Entities
' ============================================================================
PRINT "--- Creating Game Entities ---"
PRINT

CreateEntity game, "Player", 0, 10, 0
CreateEntity game, "Enemy1", 5, 5, 0
CreateEntity game, "Enemy2", -5, 8, 0
CreateEntity game, "Powerup", 0, 0, 0

PRINT "Created "; game.EntityCount; " entities:"
DIM i AS LONG
FOR i = 0 TO game.Entities.Count - 1
    DIM entityStr AS STRING
    DIM entityName AS STRING
    DIM entityPos AS QBNex_Vec3
    
    entityStr = List_Get(game.Entities, i)
    ParseEntity entityStr, entityName, entityPos
    
    PRINT "  ["; i; "] "; RTRIM$(entityName);
    PRINT " at ("; INT(entityPos.X * 10) / 10;
    PRINT ","; INT(entityPos.Y * 10) / 10;
    PRINT ","; INT(entityPos.Z * 10) / 10; ")"
NEXT i
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' Simulation Loop
' ============================================================================
PRINT "--- Running Physics Simulation ---"
PRINT

DIM playerPos AS QBNex_Vec3
DIM playerVel AS QBNex_Vec3
Vec3_Set playerPos, 0, 10, 0
Vec3_Set playerVel, 1, 0, 0

DIM frame AS LONG
FOR frame = 1 TO 10
    PRINT "Frame "; frame; ":"
    
    ' Apply physics
    ApplyGravity playerVel, game.DeltaTime
    UpdatePosition playerPos, playerVel, game.DeltaTime
    
    ' Display state
    PRINT "  Position: (";
    PRINT INT(playerPos.X * 100) / 100; ",";
    PRINT INT(playerPos.Y * 100) / 100; ",";
    PRINT INT(playerPos.Z * 100) / 100; ")"
    
    PRINT "  Velocity: (";
    PRINT INT(playerVel.X * 100) / 100; ",";
    PRINT INT(playerVel.Y * 100) / 100; ",";
    PRINT INT(playerVel.Z * 100) / 100; ")"
    
    PRINT "  Speed: "; INT(Vec3_Length(playerVel) * 100) / 100
    
    ' Check ground collision
    IF playerPos.Y <= 0 THEN
        playerPos.Y = 0
        playerVel.Y = 0
        PRINT "  *** Ground collision! ***"
    END IF
    
    game.TotalTime = game.TotalTime + game.DeltaTime
    PRINT
NEXT frame

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' Vector Math Demonstration
' ============================================================================
PRINT "--- Vector Math Demonstration ---"
PRINT

DIM v1 AS QBNex_Vec3, v2 AS QBNex_Vec3, result AS QBNex_Vec3

Vec3_Set v1, 3, 4, 0
Vec3_Set v2, 1, 0, 0

PRINT "Vector 1: ("; v1.X; ","; v1.Y; ","; v1.Z; ")"
PRINT "Vector 2: ("; v2.X; ","; v2.Y; ","; v2.Z; ")"
PRINT

Vec3_Add result, v1, v2
PRINT "Add: ("; result.X; ","; result.Y; ","; result.Z; ")"

Vec3_Sub result, v1, v2
PRINT "Subtract: ("; result.X; ","; result.Y; ","; result.Z; ")"

PRINT "Dot product: "; Vec3_Dot(v1, v2)
PRINT "V1 Length: "; Vec3_Length(v1)
PRINT "Distance: "; Vec3_Distance(v1, v2)

Vec3_Normalize result, v1
PRINT "V1 Normalized: (";
PRINT INT(result.X * 1000) / 1000; ",";
PRINT INT(result.Y * 1000) / 1000; ",";
PRINT INT(result.Z * 1000) / 1000; ")"

Vec3_Cross result, v1, v2
PRINT "Cross product: ("; result.X; ","; result.Y; ","; result.Z; ")"
PRINT

PRINT "Press any key to continue..."
SLEEP
CLS

' ============================================================================
' Collision Detection
' ============================================================================
PRINT "--- Collision Detection ---"
PRINT

DIM obj1Pos AS QBNex_Vec3, obj2Pos AS QBNex_Vec3
Vec3_Set obj1Pos, 0, 0, 0
Vec3_Set obj2Pos, 3, 4, 0

PRINT "Object 1: ("; obj1Pos.X; ","; obj1Pos.Y; ","; obj1Pos.Z; ")"
PRINT "Object 2: ("; obj2Pos.X; ","; obj2Pos.Y; ","; obj2Pos.Z; ")"
PRINT "Distance: "; Vec3_Distance(obj1Pos, obj2Pos)
PRINT

DIM collisionRadius AS DOUBLE
collisionRadius = 6

IF CheckCollision(obj1Pos, obj2Pos, collisionRadius) THEN
    PRINT "✓ Collision detected (radius: "; collisionRadius; ")"
ELSE
    PRINT "✗ No collision (radius: "; collisionRadius; ")"
END IF
PRINT

collisionRadius = 4

IF CheckCollision(obj1Pos, obj2Pos, collisionRadius) THEN
    PRINT "✓ Collision detected (radius: "; collisionRadius; ")"
ELSE
    PRINT "✗ No collision (radius: "; collisionRadius; ")"
END IF
PRINT

' ============================================================================
' Game Statistics
' ============================================================================
PRINT "--- Game Statistics ---"
PRINT

DIM stats AS QBNex_Dict
Dict_Init stats

Dict_Set stats, "total_entities", LTRIM$(STR$(game.EntityCount))
Dict_Set stats, "total_time", LTRIM$(STR$(INT(game.TotalTime * 100) / 100))
Dict_Set stats, "frame_time", LTRIM$(STR$(game.DeltaTime))
Dict_Set stats, "fps", LTRIM$(STR$(INT(1 / game.DeltaTime)))

PRINT "Total Entities: "; Dict_Get(stats, "total_entities")
PRINT "Total Time: "; Dict_Get(stats, "total_time"); " seconds"
PRINT "Frame Time: "; Dict_Get(stats, "frame_time"); " seconds"
PRINT "Target FPS: "; Dict_Get(stats, "fps")
PRINT

' ============================================================================
' Cleanup
' ============================================================================
List_Free game.Entities
Dict_Free stats

PRINT "========================================================================"
PRINT "Game Engine Example Complete!"
PRINT "========================================================================"
PRINT
PRINT "This example demonstrated:"
PRINT "  • Entity management with Lists"
PRINT "  • Vector math for positions and velocities"
PRINT "  • Physics simulation (gravity, movement)"
PRINT "  • Collision detection"
PRINT "  • Game state management"
PRINT "  • Statistics tracking with Dictionary"
PRINT
PRINT "These concepts form the foundation of a simple game engine!"
PRINT
PRINT "Press any key to exit..."
SLEEP
