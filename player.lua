local player = {}
player.__index = player

local function rectsOverlap(a, b)
    return a.x < b.x + b.width and a.x + a.width > b.x and
        a.y < b.y + b.height and a.y + a.height > b.y
end

function player.create(x, y)
    return setmetatable({
        x = x or 0,
        y = y or 0,
        radius = 15,
        speed = 230,
        sprintSpeed = 330,
        fireCooldown = 0,
        fireRate = 0.12,
        health = 100,
        aimX = 1,
        aimY = 0,
    }, player)
end

function player:center()
    return self.x, self.y
end

function player:getRect(x, y)
    return {
        x = (x or self.x) - self.radius,
        y = (y or self.y) - self.radius,
        width = self.radius * 2,
        height = self.radius * 2,
    }
end

function player:update(dt, walls)
    local dx, dy = 0, 0

    if love.keyboard.isDown("w", "up") then dy = dy - 1 end
    if love.keyboard.isDown("s", "down") then dy = dy + 1 end
    if love.keyboard.isDown("a", "left") then dx = dx - 1 end
    if love.keyboard.isDown("d", "right") then dx = dx + 1 end

    if dx ~= 0 or dy ~= 0 then
        local length = math.sqrt(dx * dx + dy * dy)
        dx, dy = dx / length, dy / length
    end

    local speed = love.keyboard.isDown("lshift", "rshift") and self.sprintSpeed or self.speed
    self:move(dx * speed * dt, dy * speed * dt, walls)

    local mx, my = love.mouse.getPosition()
    local ax, ay = mx - self.x, my - self.y
    local length = math.sqrt(ax * ax + ay * ay)
    if length > 0 then
        self.aimX, self.aimY = ax / length, ay / length
    end

    self.fireCooldown = math.max(0, self.fireCooldown - dt)
end

function player:move(dx, dy, walls)
    local nextX = self.x + dx
    local rectX = self:getRect(nextX, self.y)
    for _, wall in ipairs(walls) do
        if rectsOverlap(rectX, wall) then
            nextX = self.x
            break
        end
    end
    self.x = nextX

    local nextY = self.y + dy
    local rectY = self:getRect(self.x, nextY)
    for _, wall in ipairs(walls) do
        if rectsOverlap(rectY, wall) then
            nextY = self.y
            break
        end
    end
    self.y = nextY
end

function player:canShoot()
    return self.fireCooldown <= 0
end

function player:shoot()
    self.fireCooldown = self.fireRate
    return {
        x = self.x + self.aimX * (self.radius + 6),
        y = self.y + self.aimY * (self.radius + 6),
        vx = self.aimX * 720,
        vy = self.aimY * 720,
        radius = 4,
        life = 1.2,
    }
end

function player:draw()
    love.graphics.setColor(0.12, 0.18, 0.22)
    love.graphics.circle("fill", self.x, self.y, self.radius + 3)
    love.graphics.setColor(0.25, 0.85, 1.0)
    love.graphics.circle("fill", self.x, self.y, self.radius)

    love.graphics.setLineWidth(5)
    love.graphics.setColor(0.9, 0.98, 1.0)
    love.graphics.line(
        self.x,
        self.y,
        self.x + self.aimX * 28,
        self.y + self.aimY * 28
    )
    love.graphics.setLineWidth(1)
end

return player
