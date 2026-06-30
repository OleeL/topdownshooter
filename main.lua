local playerFactory = require("player")
local wallFactory = require("wall")

local WIDTH, HEIGHT = 960, 640
local player
local walls = {}
local bullets = {}
local enemies = {}
local particles = {}
local bodies = {}
local splats = {}
local spawnTimer = 0
local score = 0
local gameOver = false

local EPSILON = 0.0001

local function atan2(y, x)
    if math.atan2 then
        return math.atan2(y, x)
    end

    if x > 0 then
        return math.atan(y / x)
    elseif x < 0 and y >= 0 then
        return math.atan(y / x) + math.pi
    elseif x < 0 then
        return math.atan(y / x) - math.pi
    elseif y > 0 then
        return math.pi * 0.5
    elseif y < 0 then
        return -math.pi * 0.5
    end

    return 0
end

local function clamp(value, min, max)
    return math.max(min, math.min(max, value))
end

local function distance(a, b, x, y)
    local dx = a - x
    local dy = b - y
    return math.sqrt(dx * dx + dy * dy)
end

local function circleRectOverlap(circle, rect)
    local closestX = clamp(circle.x, rect.x, rect.x + rect.width)
    local closestY = clamp(circle.y, rect.y, rect.y + rect.height)
    return distance(circle.x, circle.y, closestX, closestY) < circle.radius
end

local function addBody(enemy)
    table.insert(bodies, {
        x = enemy.x,
        y = enemy.y,
        radius = enemy.radius,
        rotation = love.math.random() * math.pi * 2,
    })

    local dots = {}
    for _ = 1, love.math.random(7, 12) do
        local angle = love.math.random() * math.pi * 2
        local spread = love.math.random(6, 26)
        table.insert(dots, {
            x = math.cos(angle) * spread,
            y = math.sin(angle) * spread,
            radius = love.math.random(3, 9),
        })
    end

    table.insert(splats, {
        x = enemy.x,
        y = enemy.y,
        rotation = love.math.random() * math.pi * 2,
        dots = dots,
    })
end

local function pushEnemy(enemy, bullet)
    local length = math.sqrt(bullet.vx * bullet.vx + bullet.vy * bullet.vy)
    if length <= 0 then return end

    local push = 12
    local nextX = enemy.x + bullet.vx / length * push
    local nextY = enemy.y + bullet.vy / length * push
    local moved = { x = nextX, y = nextY, radius = enemy.radius }

    for _, wall in ipairs(walls) do
        if circleRectOverlap(moved, wall) then
            return
        end
    end

    enemy.x = nextX
    enemy.y = nextY
end

local function addWall(x, y, width, height)
    table.insert(walls, wallFactory.create(x, y, width, height))
end

local function resetGame()
    walls = {}
    bullets = {}
    enemies = {}
    particles = {}
    bodies = {}
    splats = {}
    spawnTimer = 0
    score = 0
    gameOver = false

    addWall(-20, -20, WIDTH + 40, 20)
    addWall(-20, HEIGHT, WIDTH + 40, 20)
    addWall(-20, 0, 20, HEIGHT)
    addWall(WIDTH, 0, 20, HEIGHT)

    addWall(160, 120, 220, 28)
    addWall(620, 110, 180, 28)
    addWall(130, 420, 220, 28)
    addWall(610, 445, 240, 28)
    addWall(440, 230, 32, 190)
    addWall(505, 260, 32, 140)

    player = playerFactory.create(WIDTH * 0.5, HEIGHT * 0.78)
end

local function spawnEnemy()
    local side = love.math.random(1, 4)
    local x, y

    if side == 1 then
        x, y = love.math.random(30, WIDTH - 30), 30
    elseif side == 2 then
        x, y = love.math.random(30, WIDTH - 30), HEIGHT - 30
    elseif side == 3 then
        x, y = 30, love.math.random(30, HEIGHT - 30)
    else
        x, y = WIDTH - 30, love.math.random(30, HEIGHT - 30)
    end

    table.insert(enemies, {
        x = x,
        y = y,
        radius = 14,
        speed = love.math.random(75, 115),
        health = 2,
        biteCooldown = 0,
    })
end

local function lineIntersection(rayX, rayY, rayDX, rayDY, x1, y1, x2, y2)
    local sx, sy = x2 - x1, y2 - y1
    local denominator = rayDX * sy - rayDY * sx

    if math.abs(denominator) < EPSILON then
        return nil
    end

    local qx, qy = x1 - rayX, y1 - rayY
    local t = (qx * sy - qy * sx) / denominator
    local u = (qx * rayDY - qy * rayDX) / denominator

    if t >= 0 and u >= 0 and u <= 1 then
        return rayX + rayDX * t, rayY + rayDY * t, t
    end

    return nil
end

local function wallSegments()
    local segments = {
        { 0,     0,      WIDTH, 0 },
        { WIDTH, 0,      WIDTH, HEIGHT },
        { WIDTH, HEIGHT, 0,     HEIGHT },
        { 0,     HEIGHT, 0,     0 },
    }

    for _, wall in ipairs(walls) do
        local x, y = wall.x, wall.y
        local right, bottom = wall.x + wall.width, wall.y + wall.height
        table.insert(segments, { x, y, right, y })
        table.insert(segments, { right, y, right, bottom })
        table.insert(segments, { right, bottom, x, bottom })
        table.insert(segments, { x, bottom, x, y })
    end

    return segments
end

local function visibilityPolygon(lightX, lightY, radius)
    local segments = wallSegments()
    local angles = {}

    for _, segment in ipairs(segments) do
        for i = 1, 3, 2 do
            local angle = atan2(segment[i + 1] - lightY, segment[i] - lightX)
            table.insert(angles, angle - 0.0006)
            table.insert(angles, angle)
            table.insert(angles, angle + 0.0006)
        end
    end

    local hits = {}
    for _, angle in ipairs(angles) do
        local rayDX, rayDY = math.cos(angle), math.sin(angle)
        local closestX = lightX + rayDX * radius
        local closestY = lightY + rayDY * radius
        local closestT = radius

        for _, segment in ipairs(segments) do
            local hitX, hitY, t = lineIntersection(
                lightX,
                lightY,
                rayDX,
                rayDY,
                segment[1],
                segment[2],
                segment[3],
                segment[4]
            )

            if hitX and t < closestT then
                closestX, closestY, closestT = hitX, hitY, t
            end
        end

        table.insert(hits, { x = closestX, y = closestY, angle = angle })
    end

    table.sort(hits, function(a, b) return a.angle < b.angle end)

    local polygon = {}
    for _, hit in ipairs(hits) do
        table.insert(polygon, hit.x)
        table.insert(polygon, hit.y)
    end

    return polygon
end

local function stencilLight(lightX, lightY, radius)
    local polygon = visibilityPolygon(lightX, lightY, radius)
    if #polygon < 6 then return false end

    love.graphics.stencil(function()
        love.graphics.polygon("fill", polygon)
    end, "replace", 1, true)

    return true
end

local function drawLightGlow(lightX, lightY, radius, colour)
    love.graphics.setBlendMode("add")
    love.graphics.setStencilTest("greater", 0)

    for i = 10, 1, -1 do
        local t = i / 10
        local falloff = (1 - t) ^ 2
        love.graphics.setColor(colour[1], colour[2], colour[3], colour[4] * falloff)
        love.graphics.circle("fill", lightX, lightY, radius * t)
    end

    love.graphics.setStencilTest()
    love.graphics.setBlendMode("alpha")
end

local function addMuzzleFlash()
    table.insert(particles, {
        x = player.x + player.aimX * 28,
        y = player.y + player.aimY * 28,
        radius = 120,
        life = 0.08,
        maxLife = 0.08,
    })
end

local function updateBullets(dt)
    for i = #bullets, 1, -1 do
        local bullet = bullets[i]
        bullet.x = bullet.x + bullet.vx * dt
        bullet.y = bullet.y + bullet.vy * dt
        bullet.life = bullet.life - dt

        local removed = bullet.life <= 0 or bullet.x < 0 or bullet.x > WIDTH or bullet.y < 0 or bullet.y > HEIGHT
        for _, wall in ipairs(walls) do
            if not removed and circleRectOverlap(bullet, wall) then
                removed = true
            end
        end

        for j = #enemies, 1, -1 do
            local enemy = enemies[j]
            if not removed and distance(bullet.x, bullet.y, enemy.x, enemy.y) < bullet.radius + enemy.radius then
                pushEnemy(enemy, bullet)
                enemy.health = enemy.health - 1
                removed = true
                if enemy.health <= 0 then
                    addBody(enemy)
                    score = score + 10
                    table.remove(enemies, j)
                end
            end
        end

        if removed then
            table.remove(bullets, i)
        end
    end
end

local function updateEnemies(dt)
    for _, enemy in ipairs(enemies) do
        local dx, dy = player.x - enemy.x, player.y - enemy.y
        local length = math.sqrt(dx * dx + dy * dy)
        if length > 0 then
            dx, dy = dx / length, dy / length
        end

        local nextX = enemy.x + dx * enemy.speed * dt
        local nextY = enemy.y + dy * enemy.speed * dt
        local circleX = { x = nextX, y = enemy.y, radius = enemy.radius }
        local circleY = { x = enemy.x, y = nextY, radius = enemy.radius }

        local blockedX, blockedY = false, false
        for _, wall in ipairs(walls) do
            blockedX = blockedX or circleRectOverlap(circleX, wall)
            blockedY = blockedY or circleRectOverlap(circleY, wall)
        end

        if not blockedX then enemy.x = nextX end
        if not blockedY then enemy.y = nextY end

        enemy.biteCooldown = math.max(0, enemy.biteCooldown - dt)
        if distance(enemy.x, enemy.y, player.x, player.y) < enemy.radius + player.radius + 4 and enemy.biteCooldown <= 0 then
            player.health = player.health - 12
            enemy.biteCooldown = 0.55
            if player.health <= 0 then
                gameOver = true
            end
        end
    end
end

local function updateParticles(dt)
    for i = #particles, 1, -1 do
        local particle = particles[i]
        particle.life = particle.life - dt
        if particle.life <= 0 then
            table.remove(particles, i)
        end
    end
end

function love.load()
    love.window.setMode(WIDTH, HEIGHT, { vsync = true })
    love.window.setTitle("Ray Trace Shooter")
    love.mouse.setVisible(false)
    love.graphics.setBackgroundColor(0.02, 0.025, 0.035)
    resetGame()
end

function love.update(dt)
    if gameOver then
        if love.keyboard.isDown("r") then
            resetGame()
        end
        return
    end

    player:update(dt, walls)

    if love.mouse.isDown(1) and player:canShoot() then
        table.insert(bullets, player:shoot())
        addMuzzleFlash()
    end

    spawnTimer = spawnTimer - dt
    if spawnTimer <= 0 then
        spawnEnemy()
        spawnTimer = math.max(0.45, 1.3 - score * 0.006)
    end

    updateBullets(dt)
    updateEnemies(dt)
    updateParticles(dt)
end

local function isInPlayerSight(x, y)
    local dx, dy = x - player.x, y - player.y
    local length = math.sqrt(dx * dx + dy * dy)
    if length > 620 then return false end
    if length <= 0 then return true end

    dx, dy = dx / length, dy / length

    for _, segment in ipairs(wallSegments()) do
        local _, _, t = lineIntersection(
            player.x,
            player.y,
            dx,
            dy,
            segment[1],
            segment[2],
            segment[3],
            segment[4]
        )

        if t and t < length then
            return false
        end
    end

    return true
end

local function drawWorld()
    love.graphics.setColor(0.07, 0.08, 0.1)
    love.graphics.rectangle("fill", 0, 0, WIDTH, HEIGHT)

    love.graphics.setColor(0.1, 0.12, 0.14)
    for x = 0, WIDTH, 40 do
        love.graphics.line(x, 0, x, HEIGHT)
    end
    for y = 0, HEIGHT, 40 do
        love.graphics.line(0, y, WIDTH, y)
    end

    for _, wall in ipairs(walls) do
        love.graphics.setColor(0.34, 0.34, 0.37)
        love.graphics.rectangle("fill", wall.x, wall.y, wall.width, wall.height)
        love.graphics.setColor(0.55, 0.56, 0.6)
        love.graphics.rectangle("line", wall.x, wall.y, wall.width, wall.height)
    end

    for _, splat in ipairs(splats) do
        love.graphics.push()
        love.graphics.translate(splat.x, splat.y)
        love.graphics.rotate(splat.rotation)
        love.graphics.setColor(0.35, 0.0, 0.015, 0.78)
        for _, dot in ipairs(splat.dots) do
            love.graphics.circle("fill", dot.x, dot.y, dot.radius)
        end
        love.graphics.pop()
    end

    for _, body in ipairs(bodies) do
        love.graphics.push()
        love.graphics.translate(body.x, body.y)
        love.graphics.rotate(body.rotation)
        love.graphics.setColor(0.18, 0.025, 0.03)
        love.graphics.ellipse("fill", 0, 0, body.radius + 5, body.radius - 1)
        love.graphics.setColor(0.42, 0.05, 0.055)
        love.graphics.ellipse("fill", 0, 0, body.radius + 1, body.radius - 4)
        love.graphics.pop()
    end

    for _, bullet in ipairs(bullets) do
        love.graphics.setColor(1.0, 0.86, 0.35)
        love.graphics.circle("fill", bullet.x, bullet.y, bullet.radius)
    end

    for _, enemy in ipairs(enemies) do
        if isInPlayerSight(enemy.x, enemy.y) then
            love.graphics.setColor(0.35, 0.02, 0.04)
            love.graphics.circle("fill", enemy.x, enemy.y, enemy.radius + 3)
            love.graphics.setColor(1.0, 0.18, 0.16)
            love.graphics.circle("fill", enemy.x, enemy.y, enemy.radius)
        end
    end

    player:draw()
end

local function drawLighting()
    love.graphics.setBlendMode("alpha")
    love.graphics.setStencilTest()

    stencilLight(player.x, player.y, 620)

    for _, particle in ipairs(particles) do
        stencilLight(particle.x, particle.y, particle.radius)
    end

    for _, bullet in ipairs(bullets) do
        stencilLight(bullet.x, bullet.y, 95)
    end

    love.graphics.setStencilTest("equal", 0)
    love.graphics.setColor(0.0, 0.0, 0.0, 0.62)
    love.graphics.rectangle("fill", 0, 0, WIDTH, HEIGHT)
    love.graphics.setStencilTest()

    drawLightGlow(player.x, player.y, 260, { 0.25, 0.42, 0.58, 0.12 })

    for _, particle in ipairs(particles) do
        local t = particle.life / particle.maxLife
        drawLightGlow(particle.x, particle.y, particle.radius, { 1.0, 0.48, 0.12, 0.18 * t })
    end

    for _, bullet in ipairs(bullets) do
        drawLightGlow(bullet.x, bullet.y, 70, { 1.0, 0.62, 0.16, 0.05 })
    end

    love.graphics.setBlendMode("alpha")
    love.graphics.setStencilTest()
end

local function drawHud()
    love.graphics.setColor(1, 1, 1)
    love.graphics.print("WASD/Arrows move  Mouse aim  Hold LMB shoot  Shift sprint", 14, 12)
    love.graphics.print("Score: " .. score, 14, 32)

    love.graphics.setColor(0.18, 0.02, 0.03)
    love.graphics.rectangle("fill", 14, 56, 204, 16)
    love.graphics.setColor(0.9, 0.16, 0.18)
    love.graphics.rectangle("fill", 16, 58, 2 * clamp(player.health, 0, 100), 12)
    love.graphics.setColor(1, 1, 1)
    love.graphics.rectangle("line", 14, 56, 204, 16)

    local mx, my = love.mouse.getPosition()
    love.graphics.setColor(1, 1, 1, 0.8)
    love.graphics.circle("line", mx, my, 8)
    love.graphics.line(mx - 13, my, mx - 4, my)
    love.graphics.line(mx + 4, my, mx + 13, my)
    love.graphics.line(mx, my - 13, mx, my - 4)
    love.graphics.line(mx, my + 4, mx, my + 13)

    if gameOver then
        love.graphics.setColor(0, 0, 0, 0.72)
        love.graphics.rectangle("fill", 0, 0, WIDTH, HEIGHT)
        love.graphics.setColor(1, 1, 1)
        love.graphics.printf("GAME OVER", 0, HEIGHT * 0.42, WIDTH, "center")
        love.graphics.printf("Press R to restart", 0, HEIGHT * 0.48, WIDTH, "center")
    end
end

function love.draw()
    drawWorld()
    drawLighting()
    drawHud()
end

function love.keypressed(key)
    if key == "escape" then
        love.event.quit()
    elseif key == "r" and gameOver then
        resetGame()
    end
end
